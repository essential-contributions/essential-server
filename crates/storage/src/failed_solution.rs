use essential_types::{solution::Solution, Signed};

/// Reasons why a solution failed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SolutionFailReason {
    /// Constraint check failed.
    ConstraintsFailed,
    /// Not composable with other solutions to build a batch.
    NotComposable,
}

/// A failed solution.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FailedSolution {
    /// The failed solution.
    pub solution: Signed<Solution>,
    /// Reason why the solution failed.
    pub reason: SolutionFailReason,
}
