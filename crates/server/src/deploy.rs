use essential_check as check;
use essential_storage::Storage;
use essential_types::{intent, ContentAddress, StorageLayout};

#[cfg(test)]
mod tests;

/// Validates an intent and deploys it to storage.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, err(level=tracing::Level::DEBUG), ret(Display)))]
pub async fn deploy<S>(storage: &S, intent_set: intent::SignedSet) -> anyhow::Result<ContentAddress>
where
    S: Storage,
{
    check::intent::check_signed_set(&intent_set)?;
    let intent_set_addr = essential_hash::intent_set_addr::from_intents(&intent_set.set);

    match storage.insert_intent_set(StorageLayout, intent_set).await {
        Ok(()) => Ok(intent_set_addr),
        Err(err) => anyhow::bail!("Failed to deploy intent set: {}", err),
    }
}
