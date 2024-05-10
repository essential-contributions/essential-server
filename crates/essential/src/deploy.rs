use essential_check as check;
use essential_types::{intent::Intent, ContentAddress, Signed, StorageLayout};
use storage::Storage;

#[cfg(test)]
mod tests;

/// Validates an intent and deploys it to storage.
pub async fn deploy<S>(storage: &S, intent: Signed<Vec<Intent>>) -> anyhow::Result<ContentAddress>
where
    S: Storage,
{
    check::intent::check_signed_set(&intent)?;
    let intent_hash = essential_hash::hash(&intent.data);

    match storage.insert_intent_set(StorageLayout, intent).await {
        Ok(()) => Ok(ContentAddress(intent_hash)),
        Err(e) => anyhow::bail!("Failed to deploy intent set: {}", e),
    }
}
