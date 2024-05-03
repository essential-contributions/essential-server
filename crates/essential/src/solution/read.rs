use anyhow::ensure;
use essential_types::{
    intent::Intent,
    solution::{PartialSolution, Solution},
    ContentAddress, IntentAddress,
};
use std::{collections::HashMap, sync::Arc};
use storage::Storage;
use utils::verify;

pub async fn read_intents_from_storage<S>(
    solution: &Solution,
    storage: &S,
) -> anyhow::Result<HashMap<IntentAddress, Arc<Intent>>>
where
    S: Storage,
{
    // TODO: consider FuturesUnordered
    let mut intents: HashMap<_, _> = HashMap::new();
    for data in &solution.data {
        let address = data.intent_to_solve.clone();
        if let Ok(Some(intent)) = storage.get_intent(&address).await {
            intents.insert(address, Arc::new(intent));
        } else {
            anyhow::bail!("Failed to retrieve intent set from storage");
        }
    }
    Ok(intents)
}

pub async fn read_partial_solutions_from_storage<S>(
    solution: &Solution,
    storage: &S,
) -> anyhow::Result<HashMap<ContentAddress, Arc<PartialSolution>>>
where
    S: Storage,
{
    let mut partial_solutions: HashMap<_, _> = HashMap::new();
    for ps_address in &solution.partial_solutions {
        if let Ok(Some(ps)) = storage.get_partial_solution(&ps_address.data).await {
            ensure!(verify(&ps));
            partial_solutions.insert(ps_address.data.clone(), Arc::new(ps.data));
        } else {
            anyhow::bail!("Failed to retrieve partial solution from storage");
        }
    }
    Ok(partial_solutions)
}
