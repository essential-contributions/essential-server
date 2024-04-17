use crate::deploy::deploy;
use essential_types::{
    intent::Intent,
    solution::{PartialSolution, PartialSolutionData, Solution, SolutionData},
    ContentAddress, IntentAddress,
};
use memory_storage::MemoryStorage;
use storage::Storage;
use test_utils::{empty::Empty, sign_with_random_keypair};

pub async fn deploy_intent(intent: Intent) -> (IntentAddress, MemoryStorage) {
    let storage = MemoryStorage::default();
    (deploy_intent_to_storage(&storage, intent).await, storage)
}

pub async fn deploy_empty_intent() -> (IntentAddress, MemoryStorage) {
    deploy_intent(Intent::empty()).await
}

pub async fn deploy_empty_intent_and_get_solution() -> (Solution, IntentAddress, MemoryStorage) {
    let (intent_address, storage) = deploy_empty_intent().await;
    let mut solution = Solution::empty();
    let mut solution_data = SolutionData::empty();
    solution_data.intent_to_solve = intent_address.clone();
    solution.data.push(solution_data);
    (solution, intent_address, storage)
}

pub async fn deploy_partial_solution_with_data_to_storage<S: Storage>(
    storage: &S,
    solution: &mut Solution,
    partial_solution_data: PartialSolutionData,
) -> (ContentAddress, Solution) {
    let partial_solution = PartialSolution {
        data: vec![partial_solution_data],
        state_mutations: Default::default(),
    };
    let partial_solution_address =
        deploy_partial_solution_to_storage(storage, partial_solution).await;
    solution.partial_solutions = vec![sign_with_random_keypair(partial_solution_address.clone())];
    (partial_solution_address, solution.to_owned())
}

async fn deploy_intent_to_storage<S: Storage>(storage: &S, intent: Intent) -> IntentAddress {
    let intent_hash = ContentAddress(utils::hash(&intent));
    let intent = sign_with_random_keypair(vec![intent]);
    let result = deploy(storage, intent).await.unwrap();
    IntentAddress {
        set: result,
        intent: intent_hash,
    }
}

async fn deploy_partial_solution_to_storage<S: Storage>(
    storage: &S,
    partial_solution: PartialSolution,
) -> ContentAddress {
    let partial_solution = sign_with_random_keypair(partial_solution);
    storage
        .insert_partial_solution_into_pool(partial_solution.clone())
        .await
        .unwrap();
    ContentAddress(utils::hash(&partial_solution.data))
}
