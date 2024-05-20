use essential_check as check;
use essential_storage::Storage;
use essential_types::{intent::Intent, ContentAddress, Signed, StorageLayout};
use tracing::Level;

#[cfg(test)]
mod tests;

/// Validates an intent and deploys it to storage.
#[tracing::instrument(skip_all, err(level=Level::DEBUG), ret(Display))]
pub async fn deploy<S>(storage: &S, intent: Signed<Vec<Intent>>) -> anyhow::Result<ContentAddress>
where
    S: Storage,
{
    check::intent::check_signed_set(&intent)?;
    let intent_hash = essential_hash::content_addr(&intent.data);

    match storage.insert_intent_set(StorageLayout, intent).await {
        Ok(()) => Ok(intent_hash),
        Err(err) => anyhow::bail!("Failed to deploy intent set: {}", err),
    }
}
