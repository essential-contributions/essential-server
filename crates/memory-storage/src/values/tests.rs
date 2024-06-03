use super::*;
use essential_storage::failed_solution::SolutionFailReason;
use essential_types::Batch;
use std::vec;
use test_utils::{
    duration_secs, intent_with_salt, sign_intent_set_with_random_keypair,
    solution_with_decision_variables,
};

fn intent_set(intents: Vec<Intent>) -> IntentSet {
    let signed = sign_intent_set_with_random_keypair(intents);
    let signature = signed.signature;
    IntentSet {
        data: signed
            .set
            .into_iter()
            .map(|intent| (ContentAddress(essential_hash::hash(&intent)), intent))
            .collect(),
        signature,
    }
}

fn list_of_intent_sets(
    intents: Vec<Vec<Intent>>,
) -> (Vec<ContentAddress>, HashMap<ContentAddress, IntentSet>) {
    let order = intents
        .iter()
        .map(essential_hash::intent_set_addr::from_intents)
        .collect();
    let map = intents
        .into_iter()
        .map(|intents| {
            let addr = essential_hash::intent_set_addr::from_intents(&intents);
            (addr, intent_set(intents))
        })
        .collect();
    (order, map)
}

fn create_blocks(blocks: Vec<(u64, crate::Block)>) -> BTreeMap<Duration, crate::Block> {
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
    let mut expected = vec![
        vec![intent_with_salt(0)],
        vec![intent_with_salt(1)],
        vec![intent_with_salt(2), intent_with_salt(3)],
    ];

    // Paging yields intents ordered by CA, so make sure we expect this order.
    for set in &mut expected {
        set.sort_by_key(essential_hash::content_addr);
    }

    let (order, intents) = list_of_intent_sets(expected.clone());

    let r = page_intents(order.iter(), &intents, 0, 1);
    assert_eq!(r, vec![expected[0].clone()]);

    let r = page_intents(order.iter(), &intents, 1, 1);
    assert_eq!(r, vec![expected[1].clone()]);

    let r = page_intents(order.iter(), &intents, 1, 2);
    assert_eq!(r, vec![expected[2].clone()]);

    let r = page_intents(order.iter(), &intents, 0, 2);
    assert_eq!(r, vec![expected[0].clone(), expected[1].clone()]);

    let r = page_intents(order.iter(), &intents, 0, 3);
    assert_eq!(r, expected);
}

#[test]
fn test_page_intents_by_time() {
    let mut expected = vec![
        vec![intent_with_salt(0)],
        vec![intent_with_salt(1)],
        vec![intent_with_salt(2), intent_with_salt(3)],
    ];

    // Paging yields intents ordered by CA, so make sure we expect this order.
    for set in &mut expected {
        set.sort_by_key(essential_hash::content_addr);
    }

    let (order, intents) = list_of_intent_sets(expected.clone());
    let order: BTreeMap<_, _> = order
        .into_iter()
        .enumerate()
        .map(|(i, v)| (duration_secs(i as u64), vec![v]))
        .collect();

    let r = page_intents_by_time(&order, &intents, duration_secs(0)..duration_secs(1), 0, 1);
    assert_eq!(r, vec![expected[0].clone()]);

    let r = page_intents_by_time(&order, &intents, duration_secs(1)..duration_secs(2), 0, 1);
    assert_eq!(r, vec![expected[1].clone()]);

    let r = page_intents_by_time(&order, &intents, duration_secs(1)..duration_secs(10), 1, 1);
    assert_eq!(r, vec![expected[2].clone()]);

    let r = page_intents_by_time(&order, &intents, duration_secs(1)..duration_secs(1), 0, 1);
    assert!(r.is_empty());
}

#[test]
fn test_page_solutions_pool() {
    let solutions_iter: Vec<_> = (0..10)
        .map(|i| solution_with_decision_variables(i as usize))
        .map(|s| (essential_hash::hash(&s), s))
        .collect();
    let solutions: HashMap<_, _> = solutions_iter.iter().cloned().collect();
    let expected: Vec<_> = solutions_iter.iter().map(|(_, s)| s.clone()).collect();

    let r = page_solutions(
        solutions_iter.iter().map(|(h, _)| h),
        |h| solutions.get(h).cloned(),
        0,
        1,
    );
    assert_eq!(&r[..], &expected[0..1]);

    let r = page_solutions(
        solutions_iter.iter().map(|(h, _)| h),
        |h| solutions.get(h).cloned(),
        1,
        1,
    );
    assert_eq!(&r[..], &expected[1..=1]);

    let r = page_solutions(
        solutions_iter.iter().map(|(h, _)| h),
        |h| solutions.get(h).cloned(),
        0,
        10,
    );
    assert_eq!(&r[..], &expected[0..10]);

    let r = page_solutions(
        solutions_iter.iter().map(|(h, _)| h),
        |h| solutions.get(h).cloned(),
        1,
        6,
    );
    assert_eq!(&r[..], &expected[6..10]);
}

#[test]
fn test_paging_blocks() {
    let solutions: Vec<Vec<_>> = (0..10)
        .map(|i| {
            ((i * 10)..(i * 10 + 10))
                .map(|i| solution_with_decision_variables(i as usize))
                .map(|s| (essential_hash::hash(&s), s))
                .collect()
        })
        .collect();

    let blocks = (0..10)
        .map(|i| {
            (
                i,
                crate::Block {
                    number: i,
                    timestamp: duration_secs(i),
                    hashes: solutions[i as usize]
                        .iter()
                        .map(|(h, _)| h)
                        .copied()
                        .collect(),
                },
            )
        })
        .collect();
    let blocks = create_blocks(blocks);
    let solutions: HashMap<_, _> = solutions.into_iter().flatten().collect();
    let expected: HashMap<_, _> = blocks
        .iter()
        .map(|(d, b)| {
            (
                *d,
                essential_types::Block {
                    number: b.number,
                    timestamp: *d,
                    batch: Batch {
                        solutions: b.hashes.iter().map(|h| solutions[h].clone()).collect(),
                    },
                },
            )
        })
        .collect();

    let r = page_winning_blocks(&blocks, &solutions, None, 0, 1).unwrap();
    assert_eq!(r, vec![expected.get(&duration_secs(0)).unwrap().clone()]);

    let r = page_winning_blocks(&blocks, &solutions, None, 1, 1).unwrap();
    assert_eq!(r, vec![expected.get(&duration_secs(1)).unwrap().clone()]);

    let r = page_winning_blocks(&blocks, &solutions, None, 1, 2).unwrap();
    assert_eq!(
        r,
        vec![
            expected.get(&duration_secs(2)).unwrap().clone(),
            expected.get(&duration_secs(3)).unwrap().clone()
        ]
    );

    let r = page_winning_blocks(
        &blocks,
        &solutions,
        Some(duration_secs(0)..duration_secs(10)),
        0,
        1,
    )
    .unwrap();
    assert_eq!(r, vec![expected.get(&duration_secs(0)).unwrap().clone()]);

    let r = page_winning_blocks(
        &blocks,
        &solutions,
        Some(duration_secs(0)..duration_secs(10)),
        0,
        2,
    )
    .unwrap();
    assert_eq!(
        r,
        vec![
            expected.get(&duration_secs(0)).unwrap().clone(),
            expected.get(&duration_secs(1)).unwrap().clone()
        ]
    );

    let r = page_winning_blocks(
        &blocks,
        &solutions,
        Some(duration_secs(1)..duration_secs(2)),
        0,
        2,
    )
    .unwrap();
    assert_eq!(r, vec![expected.get(&duration_secs(1)).unwrap().clone()]);

    let r = page_winning_blocks(
        &blocks,
        &solutions,
        Some(duration_secs(1)..duration_secs(1)),
        0,
        2,
    )
    .unwrap();
    assert_eq!(r, vec![]);
}

#[test]
fn test_page_solutions() {
    let solutions: Vec<_> = (0..102).map(test_utils::solution_with_all_inputs).collect();
    let solution_hashes: Vec<_> = solutions
        .iter()
        .map(|s| (essential_hash::hash(s), SolutionFailReason::NotComposable))
        .collect();
    let solutions_map: HashMap<essential_types::Hash, Solution> = solution_hashes
        .iter()
        .map(|(h, _)| *h)
        .zip(solutions)
        .collect();
    let failed = page_solutions(
        solution_hashes.into_iter(),
        |(h, r)| {
            let solution = solutions_map.get(&h).cloned()?;
            Some(essential_storage::failed_solution::FailedSolution {
                solution,
                reason: r,
            })
        },
        0,
        100,
    );
    assert_eq!(failed.len(), 100);
}
