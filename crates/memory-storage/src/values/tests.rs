use super::*;
use essential_storage::failed_solution::SolutionFailReason;
use essential_types::contract::Contract;
use std::vec;
use test_utils::{
    duration_secs, predicate_with_salt, sign_contract_with_random_keypair,
    solution_with_decision_variables,
};

fn contract_with_addr(contract: Contract) -> ContractWithAddresses {
    let signed = sign_contract_with_random_keypair(contract);
    let signature = signed.signature;
    let salt = signed.contract.salt;
    ContractWithAddresses {
        data: signed
            .contract
            .predicates
            .into_iter()
            .map(|p| essential_hash::content_addr(&p))
            .collect(),
        signature,
        salt,
    }
}

fn list_of_contracts(
    contract: Vec<Contract>,
) -> (
    Vec<ContentAddress>,
    HashMap<ContentAddress, ContractWithAddresses>,
    HashMap<ContentAddress, Predicate>,
) {
    let order = contract
        .iter()
        .map(essential_hash::contract_addr::from_contract)
        .collect();
    let map = contract
        .iter()
        .cloned()
        .map(|contract| {
            let addr = essential_hash::contract_addr::from_contract(&contract);
            (addr, contract_with_addr(contract))
        })
        .collect();
    let predicates = contract
        .into_iter()
        .flat_map(|c| {
            c.predicates
                .into_iter()
                .map(|p| (essential_hash::content_addr(&p), p))
                .collect::<HashMap<_, _>>()
        })
        .collect();
    (order, map, predicates)
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
fn test_page_contract() {
    let mut expected: Vec<Contract> = vec![
        vec![predicate_with_salt(0)].into(),
        vec![predicate_with_salt(1)].into(),
        vec![predicate_with_salt(2), predicate_with_salt(3)].into(),
    ];

    // Paging yields contract ordered by CA, so make sure we expect this order.
    for contract in &mut expected {
        contract.sort_by_key(essential_hash::content_addr);
    }

    let (order, contract, predicates) = list_of_contracts(expected.clone());

    let r = page_contract(order.iter(), &contract, &predicates, 0, 1);
    assert_eq!(r, vec![expected[0].clone()]);

    let r = page_contract(order.iter(), &contract, &predicates, 1, 1);
    assert_eq!(r, vec![expected[1].clone()]);

    let r = page_contract(order.iter(), &contract, &predicates, 1, 2);
    assert_eq!(r, vec![expected[2].clone()]);

    let r = page_contract(order.iter(), &contract, &predicates, 0, 2);
    assert_eq!(r, vec![expected[0].clone(), expected[1].clone()]);

    let r = page_contract(order.iter(), &contract, &predicates, 0, 3);
    assert_eq!(r, expected);
}

#[test]
fn test_page_contract_by_time() {
    let mut expected: Vec<Contract> = vec![
        vec![predicate_with_salt(0)].into(),
        vec![predicate_with_salt(1)].into(),
        vec![predicate_with_salt(2), predicate_with_salt(3)].into(),
    ];

    // Paging yields contract ordered by CA, so make sure we expect this order.
    for contract in &mut expected {
        contract.sort_by_key(essential_hash::content_addr);
    }

    let (order, contract, predicates) = list_of_contracts(expected.clone());
    let order: BTreeMap<_, _> = order
        .into_iter()
        .enumerate()
        .map(|(i, v)| (duration_secs(i as u64), vec![v]))
        .collect();

    let r = page_contract_by_time(
        &order,
        &contract,
        &predicates,
        duration_secs(0)..duration_secs(1),
        0,
        1,
    );
    assert_eq!(r, vec![expected[0].clone()]);

    let r = page_contract_by_time(
        &order,
        &contract,
        &predicates,
        duration_secs(1)..duration_secs(2),
        0,
        1,
    );
    assert_eq!(r, vec![expected[1].clone()]);

    let r = page_contract_by_time(
        &order,
        &contract,
        &predicates,
        duration_secs(1)..duration_secs(10),
        1,
        1,
    );
    assert_eq!(r, vec![expected[2].clone()]);

    let r = page_contract_by_time(
        &order,
        &contract,
        &predicates,
        duration_secs(1)..duration_secs(1),
        0,
        1,
    );
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
                    number: b.number as Word,
                    timestamp: *d,
                    solutions: b.hashes.iter().map(|h| solutions[h].clone()).collect(),
                },
            )
        })
        .collect();
    let block_time_index: HashMap<_, _> = blocks.iter().map(|(d, b)| (b.number, *d)).collect();

    let r = page_blocks(&blocks, &solutions, &block_time_index, None, None, 0, 1).unwrap();
    assert_eq!(r, vec![expected.get(&duration_secs(0)).unwrap().clone()]);

    let r = page_blocks(&blocks, &solutions, &block_time_index, None, None, 1, 1).unwrap();
    assert_eq!(r, vec![expected.get(&duration_secs(1)).unwrap().clone()]);

    let r = page_blocks(&blocks, &solutions, &block_time_index, None, None, 1, 2).unwrap();
    assert_eq!(
        r,
        vec![
            expected.get(&duration_secs(2)).unwrap().clone(),
            expected.get(&duration_secs(3)).unwrap().clone()
        ]
    );

    let r = page_blocks(
        &blocks,
        &solutions,
        &block_time_index,
        Some(duration_secs(0)..duration_secs(10)),
        None,
        0,
        1,
    )
    .unwrap();
    assert_eq!(r, vec![expected.get(&duration_secs(0)).unwrap().clone()]);

    let r = page_blocks(
        &blocks,
        &solutions,
        &block_time_index,
        Some(duration_secs(0)..duration_secs(10)),
        None,
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

    let r = page_blocks(
        &blocks,
        &solutions,
        &block_time_index,
        Some(duration_secs(1)..duration_secs(2)),
        None,
        0,
        2,
    )
    .unwrap();
    assert_eq!(r, vec![expected.get(&duration_secs(1)).unwrap().clone()]);

    let r = page_blocks(
        &blocks,
        &solutions,
        &block_time_index,
        Some(duration_secs(1)..duration_secs(1)),
        None,
        0,
        2,
    )
    .unwrap();
    assert_eq!(r, vec![]);

    let r = page_blocks(&blocks, &solutions, &block_time_index, None, Some(5), 0, 1).unwrap();
    assert_eq!(r, vec![expected.get(&duration_secs(5)).unwrap().clone()]);

    let r = page_blocks(&blocks, &solutions, &block_time_index, None, Some(5), 2, 1).unwrap();
    assert_eq!(r, vec![expected.get(&duration_secs(7)).unwrap().clone()]);

    let r = page_blocks(&blocks, &solutions, &block_time_index, None, Some(10), 0, 1).unwrap();
    assert_eq!(r, vec![]);

    let r = page_blocks(
        &blocks,
        &solutions,
        &block_time_index,
        Some(duration_secs(3)..duration_secs(5)),
        Some(1),
        0,
        1,
    )
    .unwrap();
    assert_eq!(r, vec![expected.get(&duration_secs(3)).unwrap().clone()]);

    let r = page_blocks(
        &blocks,
        &solutions,
        &block_time_index,
        Some(duration_secs(3)..duration_secs(5)),
        Some(4),
        0,
        1,
    )
    .unwrap();
    assert_eq!(r, vec![expected.get(&duration_secs(4)).unwrap().clone()]);

    let r = page_blocks(
        &blocks,
        &solutions,
        &block_time_index,
        Some(duration_secs(3)..duration_secs(5)),
        Some(4),
        0,
        10,
    )
    .unwrap();
    assert_eq!(r, vec![expected.get(&duration_secs(4)).unwrap().clone()]);

    let r = page_blocks(
        &blocks,
        &solutions,
        &block_time_index,
        Some(duration_secs(3)..duration_secs(5)),
        Some(6),
        0,
        10,
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
