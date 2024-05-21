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
        if let Ok(Some(intent)) = storage.get_intent(&address).await {
            intents.insert(address, Arc::new(intent));
        } else {
            anyhow::bail!("Failed to retrieve intent set from storage");
        }
    }
    #[cfg(feature = "tracing")]
    tracing::trace!(count = intents.len());
    Ok(intents)
}
