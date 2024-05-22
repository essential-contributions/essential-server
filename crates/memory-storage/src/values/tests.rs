use super::*;
use essential_types::Batch;
use std::vec;
use test_utils::{
    duration_secs, intent_with_salt, sign_with_random_keypair, solution_with_decision_variables,
};

fn intent_set(intents: Vec<Intent>) -> IntentSet {
    let order = intents
        .iter()
        .map(|intent| ContentAddress(essential_hash::hash(intent)))
        .collect();
    let signature = sign_with_random_keypair(&intents).signature;
    IntentSet {
        data: intents
            .into_iter()
            .map(|intent| (ContentAddress(essential_hash::hash(&intent)), intent))
            .collect(),
        storage_layout: essential_types::StorageLayout,
        order,
        signature,
    }
}

fn list_of_intent_sets(
    intents: Vec<Vec<Intent>>,
) -> (Vec<ContentAddress>, HashMap<ContentAddress, IntentSet>) {
    let order = intents
        .iter()
        .map(|intents| ContentAddress(essential_hash::hash(&intents)))
        .collect();
    let map = intents
        .into_iter()
        .map(|intents| {
            let address = ContentAddress(essential_hash::hash(&intents));
            (address, intent_set(intents))
        })
        .collect();
    (order, map)
}

fn create_blocks(blocks: Vec<(u64, Block)>) -> BTreeMap<Duration, Block> {
    blocks
        .into_iter()
        .map(|(number, block)| {
            let timestamp = Duration::from_secs(number);
            (timestamp, block)
        })
        .collect()
}

#[test]
fn test_page_intents() {
    let (order, intents) = list_of_intent_sets(vec![
        vec![intent_with_salt(0)],
        vec![intent_with_salt(1)],
        vec![intent_with_salt(2), intent_with_salt(3)],
    ]);

    let r = page_intents(order.iter(), &intents, 0, 1);
    assert_eq!(r, vec![vec![intent_with_salt(0)]]);

    let r = page_intents(order.iter(), &intents, 1, 1);
    assert_eq!(r, vec![vec![intent_with_salt(1)]]);

    let r = page_intents(order.iter(), &intents, 1, 2);
    assert_eq!(r, vec![vec![intent_with_salt(2), intent_with_salt(3)]]);

    let r = page_intents(order.iter(), &intents, 0, 2);
    assert_eq!(
        r,
        vec![vec![intent_with_salt(0)], vec![intent_with_salt(1)]]
    );

    let r = page_intents(order.iter(), &intents, 0, 3);
    assert_eq!(
        r,
        vec![
            vec![intent_with_salt(0)],
            vec![intent_with_salt(1)],
            vec![intent_with_salt(2), intent_with_salt(3)]
        ]
    );
}

#[test]
fn test_page_intents_by_time() {
    let (order, intents) = list_of_intent_sets(vec![
        vec![intent_with_salt(0)],
        vec![intent_with_salt(1)],
        vec![intent_with_salt(2), intent_with_salt(3)],
    ]);
    let order: BTreeMap<_, _> = order
        .into_iter()
        .enumerate()
        .map(|(i, v)| (duration_secs(i as u64), v))
        .collect();

    let r = page_intents_by_time(&order, &intents, duration_secs(0)..duration_secs(1), 0, 1);
    assert_eq!(r, vec![vec![intent_with_salt(0)]]);

    let r = page_intents_by_time(&order, &intents, duration_secs(1)..duration_secs(2), 0, 1);
    assert_eq!(r, vec![vec![intent_with_salt(1)]]);

    let r = page_intents_by_time(&order, &intents, duration_secs(1)..duration_secs(10), 1, 1);
    assert_eq!(r, vec![vec![intent_with_salt(2), intent_with_salt(3)]]);

    let r = page_intents_by_time(&order, &intents, duration_secs(1)..duration_secs(1), 0, 1);
    assert!(r.is_empty());
}

#[test]
fn test_paging_blocks() {
    let blocks = (0..10)
        .map(|i| {
            (
                i,
                Block {
                    number: i,
                    timestamp: duration_secs(i),
                    batch: Batch {
                        solutions: ((i * 10)..(i * 10 + 10))
                            .map(|i| solution_with_decision_variables(i as usize))
                            .collect(),
                    },
                },
            )
        })
        .collect();
    let blocks = create_blocks(blocks);

    let r = page_winning_blocks(&blocks, None, 0, 1);
    assert_eq!(r, vec![blocks.get(&duration_secs(0)).unwrap().clone()]);

    let r = page_winning_blocks(&blocks, None, 1, 1);
    assert_eq!(r, vec![blocks.get(&duration_secs(1)).unwrap().clone()]);

    let r = page_winning_blocks(&blocks, None, 1, 2);
    assert_eq!(
        r,
        vec![
            blocks.get(&duration_secs(2)).unwrap().clone(),
            blocks.get(&duration_secs(3)).unwrap().clone()
        ]
    );

    let r = page_winning_blocks(&blocks, Some(duration_secs(0)..duration_secs(10)), 0, 1);
    assert_eq!(r, vec![blocks.get(&duration_secs(0)).unwrap().clone()]);

    let r = page_winning_blocks(&blocks, Some(duration_secs(0)..duration_secs(10)), 0, 2);
    assert_eq!(
        r,
        vec![
            blocks.get(&duration_secs(0)).unwrap().clone(),
            blocks.get(&duration_secs(1)).unwrap().clone()
        ]
    );

    let r = page_winning_blocks(&blocks, Some(duration_secs(1)..duration_secs(2)), 0, 2);
    assert_eq!(r, vec![blocks.get(&duration_secs(1)).unwrap().clone()]);

    let r = page_winning_blocks(&blocks, Some(duration_secs(1)..duration_secs(1)), 0, 2);
    assert_eq!(r, vec![]);
}
