use crate::{
    error::{OutOfGasError, StateReadError},
    step_op_async, step_op_sync, Gas, GasLimit, Op, OpAsync, OpAsyncResult, OpKind, OpVm,
    StateRead,
};
use constraint_vm::Access;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// A VM execution future.
///
/// See the `ExecFuture::poll` docs for details on `Future` behaviour.
pub struct ExecFuture<S, F>
where
    S: StateRead,
{
    /// Store the VM in an `Option` so that we can `take` it upon future completion.
    vm: Option<OpVm>,
    /// The parsed sequence of operations.
    ops: Vec<Op>,
    /// Access to solution data.
    access: Access<'static>,
    /// Access to state reading.
    state_read: S,
    /// A function that, given a reference to an op, returns its gas cost.
    op_gas_cost: F,
    /// Gas spent during execution so far.
    gas: GasExec,
    /// In the case that the operation future is pending (i.e a state read is in
    /// progress), we store the future here.
    pending_op: Option<PendingOp<S::Error>>,
}

/// Track gas limits and expenditure for execution.
struct GasExec {
    /// The total and yield gas limits.
    limit: GasLimit,
    /// The gas threshold at which the future should yield.
    next_yield_threshold: Gas,
    /// The total gas limit.
    spent: Gas,
}

/// Encapsulates a pending operation.
struct PendingOp<E> {
    future: Pin<Box<dyn Future<Output = (OpVm, OpAsyncResult<usize, E>)>>>,
    /// Total gas that will have been spent upon completing the op.
    next_spent: Gas,
}

// TODO: Adjust this to match recommended poll time limit on supported
// validator hardware.
pub const YIELD_GAS_LIMIT: Gas = 4096;

impl<S, F> ExecFuture<S, F>
where
    S: StateRead,
{
    pub(crate) fn boxed(
        vm: OpVm,
        ops: Vec<Op>,
        access: Access<'static>,
        op_gas_cost: F,
        state_read: S,
        gas_limit: GasLimit,
    ) -> Box<Self>
    where
        F: Fn(&Op) -> Gas,
    {
        Box::new(Self {
            ops,
            access,
            state_read,
            op_gas_cost,
            vm: Some(vm),
            gas: GasExec {
                spent: 0,
                next_yield_threshold: YIELD_GAS_LIMIT,
                limit: gas_limit,
            },
            pending_op: None,
        })
    }
}

impl<S, F> Future for Box<ExecFuture<S, F>>
where
    S: 'static + Clone + StateRead,
    F: Fn(&Op) -> Gas,
{
    /// Returns the resulting state of the VM and gas spent.
    type Output = (OpVm, Result<Gas, StateReadError<S::Error>>);

    /// Attempts to make progress on the VM execution.
    ///
    /// This poll implementation steps forward the VM by the stored operations,
    /// handling synchronous and asynchronous operations differently:
    ///
    /// - For synchronous operations, it directly steps the VM to execute the
    ///   operation.
    /// - For asynchronous operations, it creates a future that will complete
    ///   the operation and temporarily takes ownership of the VM. This future
    ///   is stored in `pending_op` until it's ready.
    ///
    /// ## Allocations
    ///
    /// This method performs allocations primarily when handling asynchronous
    /// operations:
    ///
    /// - A boxed future is allocated for each asynchronous operation to be
    ///   executed. This future is `'static` and pinned, ensuring it lives long
    ///   enough and isn't moved in memory during execution.
    ///
    /// ## Yield Behavior
    ///
    /// Execution yields in two scenarios:
    ///
    /// - **Asynchronous Operations**: When an async operation is encountered,
    ///   the method yields until the operation's future is ready. This allows
    ///   other tasks to run while awaiting the asynchronous operation to
    ///   complete.
    /// - **Gas Limit Reached**: The method also yields based on a gas spending
    ///   limit (defined by `YIELD_GAS_LIMIT`). If executing an operation
    ///   causes the `gas_until_yield` counter to deplete, the method yields to
    ///   allow the scheduler to run other tasks. This prevents long or complex
    ///   sequences of operations from monopolizing CPU time.
    ///
    /// Upon yielding, the method ensures that the state of the VM and the
    /// execution context (including gas counters and any pending operations)
    /// are preserved for when the `poll` method is called again.
    ///
    /// ## Error Handling
    ///
    /// Errors encountered during operation execution result in an immediate
    /// return of `Poll::Ready(Err(...))`, encapsulating the error within a
    /// `StateReadError`. This includes errors from:
    ///
    /// - Synchronous operations that fail during their execution.
    /// - Asynchronous operations, where errors are handled once the future
    ///   resolves.
    ///
    /// ## Completion
    ///
    /// The future completes (`Poll::Ready(Ok(...))`) when all operations
    /// have been executed and no more work remains. At this point, the VM
    /// is returned in its final state. Attempting to poll the future after
    /// completion will panic.
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        // Poll the async operation future if there is one pending.
        let mut vm = match self.pending_op.as_mut() {
            None => self.vm.take().expect("future polled after completion"),
            Some(pending) => {
                let (mut vm, res) = match Pin::new(&mut pending.future).poll(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(ready) => ready,
                };

                // Drop the future now we've resumed.
                let next_spent = pending.next_spent;
                self.pending_op.take();

                // Handle the op result.
                match res {
                    Ok(new_pc) => vm.pc = new_pc,
                    Err(err) => {
                        let err = StateReadError::Op(vm.pc, err.into());
                        return Poll::Ready((vm, Err(err)));
                    }
                };

                // Update gas spent and threshold now that we've resumed.
                self.gas.spent = next_spent;
                self.gas.next_yield_threshold =
                    self.gas.spent.saturating_add(self.gas.limit.per_yield);
                vm
            }
        };

        // Step forward the virtual machine by the next operation.
        while let Some(&op) = self.ops.get(vm.pc) {
            let op_gas = (self.op_gas_cost)(&op);

            // Check that the operation wouldn't exceed gas limit.
            let next_spent = match self
                .gas
                .spent
                .checked_add(op_gas)
                .filter(|&spent| spent <= self.gas.limit.total)
                .ok_or_else(|| out_of_gas(&self.gas, op_gas))
                .map_err(|err| StateReadError::Op(vm.pc, err.into()))
            {
                Err(err) => return Poll::Ready((vm, Err(err))),
                Ok(next_spent) => next_spent,
            };

            let res = match OpKind::from(op) {
                OpKind::Sync(op) => step_op_sync(op, self.access, &mut vm),
                OpKind::Async(op) => {
                    // Async op takes ownership of the VM and returns it upon future completion.
                    let future = Box::pin(step_op_async_owned(op, self.state_read.clone(), vm));
                    self.pending_op = Some(PendingOp { future, next_spent });
                    return Poll::Pending;
                }
            };

            // Handle any errors.
            let opt_new_pc = match res {
                Ok(opt) => opt,
                Err(err) => {
                    let err = StateReadError::Op(vm.pc, err.into());
                    return Poll::Ready((vm, Err(err)));
                }
            };

            // Operation successful, so update gas spent.
            self.gas.spent = next_spent;

            // Update the program counter, or exit if we're done.
            match opt_new_pc {
                None => break,
                Some(new_pc) => vm.pc = new_pc,
            }

            // Yield if we've reached our gas limit.
            if self.gas.next_yield_threshold <= self.gas.spent {
                self.gas.next_yield_threshold = self.gas.spent.saturating_add(YIELD_GAS_LIMIT);
                self.vm = Some(vm);
                return Poll::Pending;
            }
        }

        // Completed execution successfully.
        let vm = self.vm.take().expect("future polled after completion");
        let res = Ok(self.gas.spent);
        Poll::Ready((vm, res))
    }
}

/// A version of the `step_op_async` function that takes ownership of the VM.
///
/// This ensures the returned future is `'static`, which allows the
/// `ExecFuture` to store the future until polling it returns `Ready`.
///
/// The VM is returned along with the result of the operation.
async fn step_op_async_owned<S>(
    op: OpAsync,
    state_read: S,
    mut vm: OpVm,
) -> (OpVm, OpAsyncResult<usize, S::Error>)
where
    S: StateRead,
    S::Error: 'static,
{
    let res = step_op_async(op, &state_read, &mut vm).await;
    (vm, res)
}

/// Shorthand for constructing an `OutOfGasError`.
fn out_of_gas(exec: &GasExec, op_gas: Gas) -> OutOfGasError {
    OutOfGasError {
        spent: exec.spent,
        limit: exec.limit.total,
        op_gas,
    }
}
