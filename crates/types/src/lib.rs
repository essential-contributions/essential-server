/// Utility and gas used as a result of checking a solution's state transitions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CheckSolutionOutput {
    pub utility: f64,
    pub gas: u64,
}

/// The outcome of a solution, that is:
/// - Utility if the solution was included in a block.
/// - Failure reason if solution failed constraint checking or was not composable with other solutions.
/// This may be a stringified `SolutionFailReason`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum SolutionOutcome {
    Success(u64),
    Fail(String),
}
