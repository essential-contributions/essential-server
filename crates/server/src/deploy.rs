use essential_check as check;
use essential_storage::Storage;
use essential_types::{intent::Intent, ContentAddress, Signed, StorageLayout};

#[cfg(test)]
mod tests;

/// Validates an intent and deploys it to storage.
pub async fn deploy<S>(storage: &S, intent: Signed<Vec<Intent>>) -> anyhow::Result<ContentAddress>
where
    S: Storage,
{
    check::intent::check_signed_set(&intent)?;
    let intent_hash = essential_hash::hash(&intent.data);
    let intent_hex = hex::encode(intent_hash);

    match storage.insert_intent_set(StorageLayout, intent).await {
        Ok(()) => {
            tracing::debug!("deployed intent set: 0x{}", intent_hex);
            Ok(ContentAddress(intent_hash))
        }
        Err(err) => {
            tracing::info!(
                "error deploying intent set with hash 0x{}: {}",
                intent_hex,
                err
            );
            anyhow::bail!("Failed to deploy intent set: {}", err)
        }
    }
}
