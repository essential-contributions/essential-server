use super::*;
use crate::deploy::deploy;
use test_utils::sign_with_random_keypair;

pub async fn deploy_intent<S: Storage>(storage: &S, intent: Intent) -> IntentAddress {
    let intent_hash = ContentAddress(utils::hash(&intent));
    let intent = sign_with_random_keypair(vec![intent]);
    let result = deploy(storage, intent.clone()).await.unwrap();
    IntentAddress {
        set: result,
        intent: intent_hash,
    }
}
