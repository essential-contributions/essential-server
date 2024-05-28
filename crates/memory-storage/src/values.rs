use std::{
    collections::{BTreeMap, HashMap},
    ops::Range,
    time::Duration,
};

use essential_types::{intent::Intent, Block, ContentAddress};

use crate::IntentSet;

#[cfg(test)]
mod tests;

pub fn page_intents<'a>(
    intent_hashes: impl Iterator<Item = &'a ContentAddress>,
    intents: &HashMap<ContentAddress, IntentSet>,
    page: usize,
    page_size: usize,
) -> Vec<Vec<Intent>> {
    let start = page * page_size;
    intent_hashes
        .skip(start)
        .filter_map(|v| {
            let set = intents.get(v)?;
            Some(set.intents().cloned().collect())
        })
        .take(page_size)
        .collect()
}

pub fn page_intents_by_time(
    intent_times: &BTreeMap<Duration, ContentAddress>,
    intents: &HashMap<ContentAddress, IntentSet>,
    range: Range<Duration>,
    page: usize,
    page_size: usize,
) -> Vec<Vec<Intent>> {
    let start = page * page_size;
    intent_times
        .range(range)
        .skip(start)
        .filter_map(|(_, v)| {
            let set = intents.get(v)?;
            Some(set.intents().cloned().collect())
        })
        .take(page_size)
        .collect()
}

pub fn page_winning_blocks(
    blocks: &BTreeMap<Duration, Block>,
    range: Option<Range<Duration>>,
    page: usize,
    page_size: usize,
) -> Vec<Block> {
    let start = page * page_size;
    match range {
        Some(range) => blocks
            .range(range)
            .skip(start)
            .take(page_size)
            .map(|(_, v)| v.clone())
            .collect(),
        None => blocks
            .iter()
            .skip(start)
            .take(page_size)
            .map(|(_, v)| v.clone())
            .collect(),
    }
}
