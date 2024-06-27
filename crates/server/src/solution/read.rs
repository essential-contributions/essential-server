use essential_storage::Storage;
use essential_types::{predicate::Predicate, solution::Solution, PredicateAddress};
use std::{collections::HashMap, sync::Arc};

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, err))]
pub async fn read_contract_from_storage<S>(
    solution: &Solution,
    storage: &S,
) -> anyhow::Result<HashMap<PredicateAddress, Arc<Predicate>>>
where
    S: Storage,
{
    let mut contract: HashMap<_, _> = HashMap::new();
    for data in &solution.data {
        let address = data.predicate_to_solve.clone();
        match storage.get_predicate(&address).await {
            Ok(Some(predicate)) => {
                contract.insert(address, Arc::new(predicate));
            }
            Ok(None) => {
                anyhow::bail!(
                    "Failed to retrieve contract from storage. contract: {}, predicate: {}",
                    address.contract,
                    address.predicate
                );
            }
            Err(err) => {
                anyhow::bail!(
                    "Failed to retrieve contract from storage. contract: {}, predicate: {}. Error {}",
                    address.contract,
                    address.predicate,
                    err
                );
            }
        }
    }
    #[cfg(feature = "tracing")]
    tracing::trace!(count = contract.len());
    Ok(contract)
}
