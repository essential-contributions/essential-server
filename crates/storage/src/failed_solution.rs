use std::fmt::Display;

use essential_types::{solution::Solution, Signed};

/// Reasons why a solution failed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SolutionFailReason {
    /// Constraint check failed.
    ConstraintsFailed(String),
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Outcome of a solution check.
pub enum CheckOutcome {
    /// The solution was successful in this block.
    Success(u64),
    /// The solution failed.
    Fail(SolutionFailReason),
}
/// A solution with its outcome.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SolutionOutcome {
    /// The solution.
    pub solution: Signed<Solution>,
    /// The outcome of the solution.
    pub outcome: CheckOutcome,
}

impl Display for SolutionFailReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolutionFailReason::ConstraintsFailed(reason) => {
                write!(f, "ConstraintsFailed: {}", reason)
            }
            SolutionFailReason::NotComposable => write!(f, "NotComposable"),
        }
    }
}
