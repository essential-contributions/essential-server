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
        if let Ok(Some(intent)) = storage.get_intent(&address).await {
            intents.insert(address, Arc::new(intent));
        } else {
            tracing::info!(
                "error retrieving intent 0x{} from set 0x{} from storage",
                hex::encode(address.intent.0),
                hex::encode(address.set.0),
            );

            anyhow::bail!("Failed to retrieve intent set from storage");
        }
    }
    Ok(intents)
}
