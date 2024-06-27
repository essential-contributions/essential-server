use essential_check as check;
use essential_storage::Storage;
use essential_types::{contract::SignedContract, predicate, ContentAddress};

#[cfg(test)]
mod tests;

/// Validates an predicate and deploys it to storage.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, err(level=tracing::Level::DEBUG), ret(Display)))]
pub async fn deploy<S>(storage: &S, contract: SignedContract) -> anyhow::Result<ContentAddress>
where
    S: Storage,
{
    check::predicate::check_signed_contract(&contract)?;
    let contract_addr = essential_hash::contract_addr::from_contract(&contract.contract);

    match storage.insert_contract(contract).await {
        Ok(()) => Ok(contract_addr),
        Err(err) => anyhow::bail!("Failed to deploy contract: {}", err),
    }
}
