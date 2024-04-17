use crate::validate::validate_intents;
use essential_types::{intent::Intent, ContentAddress, Signed, StorageLayout};
use storage::Storage;

#[cfg(test)]
mod tests;

pub async fn deploy<S>(storage: &S, intent: Signed<Vec<Intent>>) -> anyhow::Result<ContentAddress>
where
    S: Storage,
{
    validate_intents(&intent)?;
    let intent_hash = utils::hash(&intent.data);

    match storage.insert_intent_set(StorageLayout, intent).await {
        Ok(()) => Ok(ContentAddress(intent_hash)),
        Err(e) => anyhow::bail!("Failed to deploy intent set: {}", e),
    }
}
