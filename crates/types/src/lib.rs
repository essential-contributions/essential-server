#![deny(missing_docs)]

//! # Types for interacting with the Essential Server.

use std::collections::BTreeMap;

use essential_types::{
    contract::Contract,
    predicate::Predicate,
    solution::{Solution, SolutionData, SolutionDataIndex},
    ContentAddress, Key, PredicateAddress, StateReadBytecode, Value,
};

const ZEROED_PREDICATE: PredicateAddress = PredicateAddress {
    contract: ContentAddress([0; 32]),
    predicate: ContentAddress([0; 32]),
};

pub mod ser;

/// Utility and gas used as a result of checking a solution's state transitions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CheckSolutionOutput {
    /// The gas used by the solution.
    pub gas: u64,
}

/// The outcome of a solution, that is:
/// - Utility if the solution was included in a block.
/// - Failure reason if solution failed constraint checking or was not composable with other solutions.
///
/// This may be a stringified `SolutionFailReason`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum SolutionOutcome {
    /// The solution was successful and included in a block.
    Success(u64),
    /// The solution failed and was not included in a block.
    Fail(String),
}

/// Solution with contract read from storage that will be used for checking.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct CheckSolution {
    /// The solution to check.
    pub solution: Solution,
    /// The contracts this solution depends on.
    pub contracts: Vec<Contract>,
}

/// Query the results of running an ordered list of state read programs.
///
/// The query can be derived from a solution, or be inline.
/// The request type can be for just the keys and values read, or for the slots read
/// or both.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct QueryStateReads {
    /// The programs that read state.
    #[serde(
        serialize_with = "essential_types::serde::bytecode::serialize_vec",
        deserialize_with = "essential_types::serde::bytecode::deserialize_vec"
    )]
    pub state_read: Vec<StateReadBytecode>,
    /// The index of the solution data that is used for the state query,
    pub index: SolutionDataIndex,
    /// The solution for this query.
    pub solution: Solution,
    /// The type of results for this request.
    pub request_type: StateReadRequestType,
}

/// The type of results for this request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum StateReadRequestType {
    /// Request the keys and values that are read with the state slots.
    All(SlotsRequest),
    /// Request only the slots that are read into.
    Slots(SlotsRequest),
    /// Request only the keys and values that are read.
    Reads,
}

/// The slots that are returned for the state read request.
#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum SlotsRequest {
    /// Return both the pre and post state slots.
    #[default]
    All,
    /// Return only the pre state slots.
    Pre,
    /// Return only the post state slots.
    Post,
}

/// The output of a state read query.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum QueryStateReadsOutput {
    #[serde(
        serialize_with = "ser::serialize_map",
        deserialize_with = "ser::deserialize_map"
    )]
    /// The keys and values that were read.
    Reads(BTreeMap<ContentAddress, BTreeMap<Key, Value>>),
    /// The slots that were read into.
    Slots(Slots),
    /// The keys and values that were read and the slots that were read into.
    All(
        #[serde(
            serialize_with = "ser::serialize_map",
            deserialize_with = "ser::deserialize_map"
        )]
        BTreeMap<ContentAddress, BTreeMap<Key, Value>>,
        Slots,
    ),
    /// The state reads failed.
    Failure(String),
}

/// Pre and post state slots.
#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Slots {
    /// The pre state slots.
    pub pre: Vec<Value>,
    /// The post state slots. Read after the mutations are applied.
    pub post: Vec<Value>,
}

impl QueryStateReads {
    /// Create a query from a solution and a predicate.
    ///
    /// It is assumed the provided predicate is the predicate that the solution data
    /// at the provided index is solving. This is not checked.
    pub fn from_solution(
        mut solution: Solution,
        index: SolutionDataIndex,
        predicate: &Predicate,
        request_type: StateReadRequestType,
    ) -> Self {
        for (i, d) in solution.data.iter_mut().enumerate() {
            if i as SolutionDataIndex == index {
                continue;
            }
            d.decision_variables = Default::default();
        }
        Self {
            state_read: predicate.state_read.clone(),
            index,
            solution,
            request_type,
        }
    }

    /// Create a query that only reads external state.
    /// The predicate address is zeroed out.
    pub fn inline_empty(
        state_read: Vec<StateReadBytecode>,
        request_type: StateReadRequestType,
    ) -> Self {
        let data = SolutionData {
            predicate_to_solve: ZEROED_PREDICATE,
            decision_variables: Default::default(),
            transient_data: Default::default(),
            state_mutations: Default::default(),
        };

        Self::inline(state_read, data, request_type)
    }

    /// Create an inline query from state reads and a single solution data.
    pub fn inline(
        state_read: Vec<StateReadBytecode>,
        data: SolutionData,
        request_type: StateReadRequestType,
    ) -> Self {
        Self {
            state_read,
            index: 0,
            solution: Solution { data: vec![data] },
            request_type,
        }
    }
}

impl Default for StateReadRequestType {
    fn default() -> Self {
        Self::All(SlotsRequest::default())
    }
}
