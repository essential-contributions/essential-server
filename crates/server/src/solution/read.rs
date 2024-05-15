use essential_storage::Storage;
use essential_types::{intent::Intent, solution::Solution, IntentAddress};
use std::{collections::HashMap, sync::Arc};

#[tracing::instrument(skip_all)]
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
                tracing::debug!(
                    "intent 0x{} not in set 0x{}",
                    hex::encode(address.intent.0),
                    hex::encode(address.set.0),
                );
            }
            Err(err) => {
                tracing::info!(
                    "error retrieving intent set 0x{} from storage: {}",
                    hex::encode(address.set.0),
                    err
                );
                anyhow::bail!("Failed to retrieve intent set from storage");
            }
        }
    }
    Ok(intents)
}
