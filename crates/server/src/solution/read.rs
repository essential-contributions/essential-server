use essential_storage::Storage;
use essential_types::{intent::Intent, solution::Solution, IntentAddress};
use std::{collections::HashMap, sync::Arc};

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, err))]
pub async fn read_intents_from_storage<S>(
    solution: &Solution,
    storage: &S,
) -> anyhow::Result<HashMap<IntentAddress, Arc<Intent>>>
where
    S: Storage,
{
    let mut intents: HashMap<_, _> = HashMap::new();
    for data in &solution.data {
        let address = data.intent_to_solve.clone();
        match storage.get_intent(&address).await {
            Ok(Some(intent)) => {
                intents.insert(address, Arc::new(intent));
            }
            Ok(None) => {
                anyhow::bail!(
                    "Failed to retrieve intent set from storage. set: {}, intent: {}",
                    address.set,
                    address.intent
                );
            }
            Err(err) => {
                anyhow::bail!(
                    "Failed to retrieve intent set from storage. set: {}, intent: {}. Error {}",
                    address.set,
                    address.intent,
                    err
                );
            }
        }
    }
    #[cfg(feature = "tracing")]
    tracing::trace!(count = intents.len());
    Ok(intents)
}
