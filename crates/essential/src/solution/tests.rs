use super::*;
use crate::deploy::deploy;
use essential_types::{ContentAddress, IntentAddress};
use memory_storage::MemoryStorage;
use test_utils::{empty::Empty, sign_with_random_keypair};

pub async fn deploy_and_return_address<S>(storage: &S, intent: Signed<Vec<Intent>>) -> IntentAddress
where
    S: Storage,
{
    let intent_hash = ContentAddress(utils::hash(&intent));
    let set_address = deploy(storage, intent).await.unwrap();
    IntentAddress {
        set: set_address,
        intent: intent_hash,
    }
}

#[tokio::test]
async fn test_submit_empty_solution() {
    let storage = MemoryStorage::default();
    let solution = Solution::empty();
    let solution = sign_with_random_keypair(solution);
    let result = submit_solution(&storage, solution.clone()).await.unwrap();
    let result = storage.list_solutions_pool().await.unwrap();
    assert_eq!(result, vec![solution]);
}
