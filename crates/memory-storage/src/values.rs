use std::{
    collections::{BTreeMap, HashMap},
    ops::Range,
    time::Duration,
};

use essential_types::{
    contract::Contract, predicate::Predicate, solution::Solution, Batch, ContentAddress,
};

use crate::ContractWithAddresses;

#[cfg(test)]
mod tests;

pub fn page_contract<'a>(
    contract_hashes: impl Iterator<Item = &'a ContentAddress>,
    contract: &HashMap<ContentAddress, ContractWithAddresses>,
    predicates: &HashMap<ContentAddress, Predicate>,
    page: usize,
    page_size: usize,
) -> Vec<Contract> {
    let start = page * page_size;
    contract_hashes
        .skip(start)
        .filter_map(|v| {
            let contract = contract.get(v)?;
            Some(Contract {
                predicates: contract.predicates_owned(predicates),
                salt: contract.salt,
            })
        })
        .filter(|v| !v.is_empty())
        .take(page_size)
        .collect()
}

pub fn page_contract_by_time(
    contract_times: &BTreeMap<Duration, Vec<ContentAddress>>,
    contract: &HashMap<ContentAddress, ContractWithAddresses>,
    predicates: &HashMap<ContentAddress, Predicate>,
    range: Range<Duration>,
    page: usize,
    page_size: usize,
) -> Vec<Contract> {
    let start = page * page_size;
    contract_times
        .range(range)
        .skip(start)
        .flat_map(|(_, v)| {
            v.iter().filter_map(|v| {
                let contract = contract.get(v)?;
                Some(Contract {
                    predicates: contract.predicates_owned(predicates),
                    salt: contract.salt,
                })
            })
        })
        .filter(|v| !v.is_empty())
        .take(page_size)
        .collect()
}

pub fn page_solutions<F, S, I>(
    solution_hashes: impl Iterator<Item = I>,
    f: F,
    page: usize,
    page_size: usize,
) -> Vec<S>
where
    F: FnMut(I) -> Option<S>,
{
    let start = page * page_size;
    solution_hashes
        .skip(start)
        .filter_map(f)
        .take(page_size)
        .collect()
}

pub fn page_winning_blocks(
    blocks: &BTreeMap<Duration, super::Block>,
    solutions: &HashMap<essential_types::Hash, Solution>,
    range: Option<Range<Duration>>,
    page: usize,
    page_size: usize,
) -> anyhow::Result<Vec<essential_types::Block>> {
    let start = page * page_size;
    match range {
        Some(range) => blocks
            .range(range)
            .skip(start)
            .take(page_size)
            .map(|(_, v)| v)
            .map(|block| {
                let super::Block {
                    number,
                    timestamp,
                    hashes,
                } = block;
                let solutions = hashes
                    .iter()
                    .map(|h| solutions.get(h).cloned())
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| anyhow::anyhow!("Missing solution"))?;
                Ok(essential_types::Block {
                    number: *number,
                    timestamp: *timestamp,
                    batch: Batch { solutions },
                })
            })
            .collect(),
        None => blocks
            .iter()
            .skip(start)
            .take(page_size)
            .map(|(_, v)| v)
            .map(|block| {
                let super::Block {
                    number,
                    timestamp,
                    hashes,
                } = block;
                let solutions = hashes
                    .iter()
                    .map(|h| solutions.get(h).cloned())
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| anyhow::anyhow!("Missing solution"))?;
                Ok(essential_types::Block {
                    number: *number,
                    timestamp: *timestamp,
                    batch: Batch { solutions },
                })
            })
            .collect(),
    }
}
