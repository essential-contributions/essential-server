use crate::validate::validate_intents;
use essential_types::{intent::Intent, ContentAddress, Signed, StorageLayout};
use storage::Storage;

#[cfg(test)]
mod tests;

/// Validates an intent and deploys it to storage.
pub async fn deploy<S>(storage: &S, intent: Signed<Vec<Intent>>) -> anyhow::Result<ContentAddress>
where
    S: Storage,
{
    validate_intents(&intent)?;

    match storage
        .insert_intent_set(StorageLayout, intent.clone())
        .await
    {
        Ok(()) => Ok(ContentAddress(utils::hash(&intent.data))),
        Err(e) => anyhow::bail!("Failed to deploy intent set: {}", e),
    }
}
