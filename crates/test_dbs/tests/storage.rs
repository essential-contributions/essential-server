use essential_storage::{
    failed_solution::{CheckOutcome, FailedSolution, SolutionFailReason},
    CommitData, Storage,
};
use essential_types::{contract::Contract, ContentAddress, PredicateAddress, Word};
use pretty_assertions::assert_eq;
use test_dbs::create_test;
use test_utils::{
    empty::Empty, predicate_with_salt, predicate_with_salt_and_state,
    sign_contract_with_random_keypair, solution_with_all_inputs,
};

create_test!(insert_contract);

async fn insert_contract<S: Storage>(storage: S) {
    // Double insert is a idempotent
    let mut contract = sign_contract_with_random_keypair(vec![
        predicate_with_salt(0),
        predicate_with_salt(1),
        predicate_with_salt(2),
    ]);
    contract.contract.sort_by_key(essential_hash::content_addr);

    storage.insert_contract(contract.clone()).await.unwrap();

    storage.insert_contract(contract.clone()).await.unwrap();

    let result = storage.list_contracts(None, None).await.unwrap();
    assert_eq!(result, vec![contract.contract.clone()]);

    // Insert many contracts
    let contracts: Vec<_> = (0..10)
        .map(|i| {
            let mut contract: Contract = vec![
                predicate_with_salt(i),
                predicate_with_salt(i + 1),
                predicate_with_salt(i + 2),
            ]
            .into();
            contract.sort_by_key(essential_hash::content_addr);
            sign_contract_with_random_keypair(contract)
        })
        .collect();

    let mut expected: Vec<_> = contracts.iter().map(|s| s.contract.clone()).collect();

    for contract in &contracts {
        storage.insert_contract(contract.clone()).await.unwrap();
    }

    let result = storage.list_contracts(None, None).await.unwrap();
    assert_eq!(result, expected);

    // Insert empty contract
    storage
        .insert_contract(sign_contract_with_random_keypair(vec![]))
        .await
        .unwrap();

    // Insert contracts with storage
    let storage_contracts: Vec<_> = (0..10)
        .map(|i| {
            let mut contract: Contract = vec![
                predicate_with_salt_and_state(i, i),
                predicate_with_salt_and_state(i + 1, i + 1),
                predicate_with_salt_and_state(i + 2, i + 2),
            ]
            .into();
            contract.sort_by_key(essential_hash::content_addr);
            sign_contract_with_random_keypair(contract)
        })
        .collect();

    expected.extend(storage_contracts.iter().map(|s| s.contract.clone()));

    for contract in &storage_contracts {
        storage.insert_contract(contract.clone()).await.unwrap();
    }

    let result = storage.list_contracts(None, None).await.unwrap();
    assert_eq!(result, expected);
}

create_test!(insert_solution_into_pool);

async fn insert_solution_into_pool<S: Storage>(storage: S) {
    // Double insert is a idempotent
    let solution = solution_with_all_inputs(0);
    storage
        .insert_solution_into_pool(solution.clone())
        .await
        .unwrap();
    storage
        .insert_solution_into_pool(solution.clone())
        .await
        .unwrap();

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result, vec![solution]);

    // Insert many solutions
    let solutions: Vec<_> = (0..10).map(solution_with_all_inputs).collect();

    for solution in &solutions {
        storage
            .insert_solution_into_pool(solution.clone())
            .await
            .unwrap();
    }

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result, solutions);
}

create_test!(move_solutions_to_solved);

async fn move_solutions_to_solved<S: Storage>(storage: S) {
    let solutions: Vec<_> = (0..10).map(solution_with_all_inputs).collect();

    for solution in &solutions {
        storage
            .insert_solution_into_pool(solution.clone())
            .await
            .unwrap();
    }

    // Move none
    storage.move_solutions_to_solved(&[]).await.unwrap();

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 10);
    let result = storage.list_blocks(None, None).await.unwrap();
    assert_eq!(result.len(), 0);

    let hashes: Vec<_> = solutions.iter().map(essential_hash::hash).collect();

    // Move some
    storage
        .move_solutions_to_solved(&hashes[3..5])
        .await
        .unwrap();

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 8);
    assert_eq!(&result[0..3], &solutions[0..3]);
    assert_eq!(&result[3..8], &solutions[5..10]);

    let result = storage.list_blocks(None, None).await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(&result[0].solutions[..], &solutions[3..5]);

    // Move missing hash is noop
    let hash = essential_hash::hash(&solution_with_all_inputs(11));
    storage.move_solutions_to_solved(&[hash]).await.unwrap();

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 8);
    let result = storage.list_blocks(None, None).await.unwrap();
    assert_eq!(result.len(), 1);

    // Move some with missing hash
    storage
        .move_solutions_to_solved(&[hashes[9], hash])
        .await
        .unwrap();

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 7);
    assert_eq!(&result[0..2], &solutions[0..2]);
    assert_eq!(&result[3..7], &solutions[5..9]);

    let result = storage.list_blocks(None, None).await.unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(&result[0].solutions[..], &solutions[3..5]);
    assert_eq!(&result[1].solutions[..], &solutions[9..10]);

    // Move all
    storage.move_solutions_to_solved(&hashes).await.unwrap();
    let result = storage.list_solutions_pool(None).await.unwrap();
    assert!(result.is_empty());
    let result = storage.list_blocks(None, None).await.unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(&result[0].solutions[..], &solutions[3..5]);
    assert_eq!(&result[1].solutions[..], &solutions[9..10]);
    assert_eq!(result[2].solutions.len(), 7);
    assert_eq!(&result[2].solutions[0..3], &solutions[0..3]);
    assert_eq!(&result[2].solutions[3..7], &solutions[5..9]);

    // Move all again is noop
    storage.move_solutions_to_solved(&hashes).await.unwrap();
    let result = storage.list_solutions_pool(None).await.unwrap();
    assert!(result.is_empty());
    let result = storage.list_blocks(None, None).await.unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(&result[0].solutions[..], &solutions[3..5]);
    assert_eq!(&result[1].solutions[..], &solutions[9..10]);
    assert_eq!(result[2].solutions.len(), 7);
    assert_eq!(&result[2].solutions[0..3], &solutions[0..3]);
    assert_eq!(&result[2].solutions[3..7], &solutions[5..9]);

    assert_eq!(result[0].number, 0);
    assert_eq!(result[1].number, 1);
    assert_eq!(result[2].number, 2);
}

create_test!(get_predicate);

async fn get_predicate<S: Storage>(storage: S) {
    let mut contract = sign_contract_with_random_keypair(vec![
        predicate_with_salt(0),
        predicate_with_salt(1),
        predicate_with_salt(2),
    ]);
    contract.contract.sort_by_key(essential_hash::content_addr);

    storage.insert_contract(contract.clone()).await.unwrap();

    let mut contract2 =
        sign_contract_with_random_keypair(vec![predicate_with_salt(0), predicate_with_salt(1)]);
    contract2.contract.sort_by_key(essential_hash::content_addr);

    storage.insert_contract(contract2.clone()).await.unwrap();

    let contract_address = essential_hash::contract_addr::from_contract(&contract.contract);
    let contract_address2 = essential_hash::contract_addr::from_contract(&contract2.contract);

    // Get existing predicate
    for predicate in &contract.contract.predicates {
        let address = PredicateAddress {
            contract: contract_address.clone(),
            predicate: essential_hash::content_addr(predicate),
        };
        let result = storage.get_predicate(&address).await.unwrap().unwrap();
        assert_eq!(result, *predicate);
    }

    for predicate in &contract2.contract.predicates {
        let address = PredicateAddress {
            contract: contract_address2.clone(),
            predicate: essential_hash::content_addr(predicate),
        };
        let result = storage.get_predicate(&address).await.unwrap().unwrap();
        assert_eq!(result, *predicate);
    }

    // Get missing predicate
    let address = PredicateAddress::empty();
    let result = storage.get_predicate(&address).await.unwrap();
    assert!(result.is_none());

    let address = PredicateAddress {
        contract: contract_address.clone(),
        predicate: ContentAddress::empty(),
    };
    let result = storage.get_predicate(&address).await.unwrap();
    assert!(result.is_none());

    let address = PredicateAddress {
        contract: ContentAddress::empty(),
        predicate: essential_hash::content_addr(&contract.contract[0]),
    };
    let result = storage.get_predicate(&address).await.unwrap();
    assert!(result.is_none());

    // Wrong contract
    let address = PredicateAddress {
        contract: contract_address2.clone(),
        predicate: essential_hash::content_addr(&predicate_with_salt(2)),
    };
    let result = storage.get_predicate(&address).await.unwrap();
    assert!(result.is_none());
}

create_test!(get_contract);

async fn get_contract<S: Storage>(storage: S) {
    let mut contracts = vec![];
    for i in 0..2 {
        let mut contract = sign_contract_with_random_keypair(vec![
            predicate_with_salt(i),
            predicate_with_salt(i + 1),
            predicate_with_salt(i + 2),
        ]);
        contract.contract.sort_by_key(essential_hash::content_addr);

        storage.insert_contract(contract.clone()).await.unwrap();
        contracts.push(contract);
    }

    // Get existing contracts
    for contract in &contracts {
        let address = essential_hash::contract_addr::from_contract(&contract.contract);
        let result = storage.get_contract(&address).await.unwrap().unwrap();
        assert_eq!(result.signature, contract.signature);
        assert_eq!(result.contract, contract.contract);
    }

    // Get missing contract
    let result = storage
        .get_contract(&ContentAddress::empty())
        .await
        .unwrap();
    assert!(result.is_none());
}

create_test!(list_contracts);

async fn list_contracts<S: Storage>(storage: S) {
    // List empty
    let result = storage.list_contracts(None, None).await.unwrap();
    assert!(result.is_empty());

    let mut contracts = vec![];
    for i in 0..102 {
        let mut contract = sign_contract_with_random_keypair(vec![
            predicate_with_salt(i),
            predicate_with_salt(i + 1),
            predicate_with_salt(i + 2),
        ]);
        contract.contract.sort_by_key(essential_hash::content_addr);

        storage.insert_contract(contract.clone()).await.unwrap();
        contracts.push(contract.contract);
    }

    // List up to page size
    let result = storage.list_contracts(None, None).await.unwrap();
    assert_eq!(&result[..], &contracts[0..100]);

    // List first page
    let result = storage.list_contracts(None, Some(0)).await.unwrap();
    assert_eq!(&result[..], &contracts[0..100]);

    // List second page
    let result = storage.list_contracts(None, Some(1)).await.unwrap();
    assert_eq!(&result[..], &contracts[100..102]);

    // List empty third page
    let result = storage.list_contracts(None, Some(2)).await.unwrap();
    assert!(result.is_empty());

    // List within time
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let start = time - std::time::Duration::from_secs(100);
    let end = time + std::time::Duration::from_secs(100);
    let result = storage
        .list_contracts(Some(start..end), None)
        .await
        .unwrap();
    assert_eq!(&result[..], &contracts[0..100]);

    // List within time and page
    let result = storage
        .list_contracts(Some(start..end), Some(1))
        .await
        .unwrap();
    assert_eq!(&result[..], &contracts[100..102]);

    // List within time and empty page
    let result = storage
        .list_contracts(Some(start..end), Some(2))
        .await
        .unwrap();
    assert!(result.is_empty());

    // List outside time
    let end = time - std::time::Duration::from_secs(80);
    let result = storage
        .list_contracts(Some(start..end), None)
        .await
        .unwrap();
    assert!(result.is_empty());
}

create_test!(list_solutions_pool);

async fn list_solutions_pool<S: Storage>(storage: S) {
    let solutions: Vec<_> = (0..102).map(solution_with_all_inputs).collect();

    for solution in &solutions {
        storage
            .insert_solution_into_pool(solution.clone())
            .await
            .unwrap();
    }

    // List up to page size
    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(&result[..], &solutions[0..100]);

    // List first page
    let result = storage.list_solutions_pool(Some(0)).await.unwrap();
    assert_eq!(&result[..], &solutions[0..100]);

    // List second page
    let result = storage.list_solutions_pool(Some(1)).await.unwrap();
    assert_eq!(&result[..], &solutions[100..102]);

    // List empty third page
    let result = storage.list_solutions_pool(Some(2)).await.unwrap();
    assert!(result.is_empty());
}

create_test!(list_failed_solutions_pool);

async fn list_failed_solutions_pool<S: Storage>(storage: S) {
    // List empty
    let result = storage.list_failed_solutions_pool(None).await.unwrap();
    assert!(result.is_empty());

    let solutions: Vec<_> = (0..102).map(solution_with_all_inputs).collect();

    let mut hashes = vec![];

    for solution in &solutions {
        hashes.push((
            essential_hash::hash(solution),
            SolutionFailReason::NotComposable,
        ));
        storage
            .insert_solution_into_pool(solution.clone())
            .await
            .unwrap();
    }

    storage.move_solutions_to_failed(&hashes).await.unwrap();

    let solutions: Vec<_> = solutions
        .into_iter()
        .map(|s| FailedSolution {
            solution: s,
            reason: SolutionFailReason::NotComposable,
        })
        .collect();

    // List up to page size
    let result = storage.list_failed_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 100);
    assert_eq!(&result[..], &solutions[0..100]);

    // List first page
    let result = storage.list_failed_solutions_pool(Some(0)).await.unwrap();
    assert_eq!(&result[..], &solutions[0..100]);

    // List second page
    let result = storage.list_failed_solutions_pool(Some(1)).await.unwrap();
    assert_eq!(&result[..], &solutions[100..102]);

    // List empty third page
    let result = storage.list_failed_solutions_pool(Some(2)).await.unwrap();
    assert!(result.is_empty());
}

create_test!(list_blocks);

async fn list_blocks<S: Storage>(storage: S) {
    // Empty
    let result = storage.list_blocks(None, None).await.unwrap();
    assert!(result.is_empty());

    let solutions: Vec<_> = (0..102).map(solution_with_all_inputs).collect();

    let hashes: Vec<_> = solutions.iter().map(essential_hash::hash).collect();

    for solution in &solutions {
        storage
            .insert_solution_into_pool(solution.clone())
            .await
            .unwrap();
    }

    // Move one
    storage
        .move_solutions_to_solved(&hashes[0..1])
        .await
        .unwrap();

    let result = storage.list_blocks(None, None).await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].solutions.len(), 1);
    assert_eq!(result[0].solutions[0], solutions[0]);
    assert_eq!(result[0].number, 0);

    // Move rest
    for i in 1..102 {
        storage
            .move_solutions_to_solved(&hashes[i..i + 1])
            .await
            .unwrap();
    }

    // List up to page size
    let result = storage.list_blocks(None, None).await.unwrap();
    assert_eq!(result.len(), 100);

    assert_eq!(result[0].solutions.len(), 1);
    assert_eq!(result[0].solutions[0], solutions[0]);
    assert_eq!(result[0].number, 0);

    assert_eq!(result[99].solutions.len(), 1);
    assert_eq!(result[99].solutions[0], solutions[99]);
    assert_eq!(result[99].number, 99);

    // List first page
    let result = storage.list_blocks(None, Some(0)).await.unwrap();
    assert_eq!(result.len(), 100);

    assert_eq!(result[0].solutions.len(), 1);
    assert_eq!(result[0].solutions[0], solutions[0]);
    assert_eq!(result[0].number, 0);

    assert_eq!(result[99].solutions.len(), 1);
    assert_eq!(result[99].solutions[0], solutions[99]);
    assert_eq!(result[99].number, 99);

    // List second page
    let result = storage.list_blocks(None, Some(1)).await.unwrap();
    assert_eq!(result.len(), 2);

    assert_eq!(result[0].solutions.len(), 1);
    assert_eq!(result[0].solutions[0], solutions[100]);
    assert_eq!(result[0].number, 100);

    assert_eq!(result[1].solutions.len(), 1);
    assert_eq!(result[1].solutions[0], solutions[101]);
    assert_eq!(result[1].number, 101);

    // List empty third page
    let result = storage.list_blocks(None, Some(2)).await.unwrap();
    assert_eq!(result.len(), 0);

    // List within time
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let start = time - std::time::Duration::from_secs(100);
    let end = time + std::time::Duration::from_secs(100);

    let result = storage.list_blocks(Some(start..end), None).await.unwrap();
    assert_eq!(result.len(), 100);

    assert_eq!(result[0].solutions.len(), 1);
    assert_eq!(result[0].solutions[0], solutions[0]);
    assert_eq!(result[0].number, 0);

    assert_eq!(result[99].solutions.len(), 1);
    assert_eq!(result[99].solutions[0], solutions[99]);
    assert_eq!(result[99].number, 99);

    // List within time and page
    let result = storage
        .list_blocks(Some(start..end), Some(1))
        .await
        .unwrap();
    assert_eq!(result.len(), 2);

    assert_eq!(result[0].solutions.len(), 1);
    assert_eq!(result[0].solutions[0], solutions[100]);
    assert_eq!(result[0].number, 100);

    assert_eq!(result[1].solutions.len(), 1);
    assert_eq!(result[1].solutions[0], solutions[101]);
    assert_eq!(result[1].number, 101);

    // List within time and empty page
    let result = storage
        .list_blocks(Some(start..end), Some(2))
        .await
        .unwrap();
    assert_eq!(result.len(), 0);

    // List outside time and empty page
    let end = time - std::time::Duration::from_secs(80);
    let result = storage.list_blocks(Some(start..end), None).await.unwrap();
    assert_eq!(result.len(), 0);
}

create_test!(get_solution);

async fn get_solution<S: Storage>(storage: S) {
    let solutions: Vec<_> = (0..3).map(solution_with_all_inputs).collect();
    let hashes: Vec<_> = solutions.iter().map(essential_hash::hash).collect();

    for solution in &solutions {
        storage
            .insert_solution_into_pool(solution.clone())
            .await
            .unwrap();
    }

    storage
        .move_solutions_to_solved(&hashes[1..2])
        .await
        .unwrap();
    storage
        .move_solutions_to_failed(&[(hashes[2], SolutionFailReason::NotComposable)])
        .await
        .unwrap();

    let r = storage.list_blocks(None, None).await.unwrap();
    assert_eq!(r.len(), 1);

    // Get existing solution in pool
    let result = storage.get_solution(hashes[0]).await.unwrap().unwrap();
    assert_eq!(result.solution, solutions[0]);
    assert!(result.outcome.is_empty());

    // Get existing solution in solved
    let result = storage.get_solution(hashes[1]).await.unwrap().unwrap();
    assert_eq!(result.solution, solutions[1]);
    assert_eq!(result.outcome.len(), 1);
    assert_eq!(result.outcome[0], CheckOutcome::Success(0));

    // Get existing solution in failed
    let result = storage.get_solution(hashes[2]).await.unwrap().unwrap();
    assert_eq!(result.solution, solutions[2]);
    assert_eq!(result.outcome.len(), 1);
    assert_eq!(
        result.outcome[0],
        CheckOutcome::Fail(SolutionFailReason::NotComposable)
    );

    // Get missing solution
    let result = storage
        .get_solution(ContentAddress::empty().0)
        .await
        .unwrap();
    assert!(result.is_none());

    for solution in &solutions {
        storage
            .insert_solution_into_pool(solution.clone())
            .await
            .unwrap();
    }

    storage
        .move_solutions_to_solved(&hashes[2..3])
        .await
        .unwrap();
    storage
        .move_solutions_to_failed(&[(hashes[1], SolutionFailReason::NotComposable)])
        .await
        .unwrap();
    let r = storage.list_blocks(None, None).await.unwrap();
    assert_eq!(r.len(), 2);

    for solution in &solutions {
        storage
            .insert_solution_into_pool(solution.clone())
            .await
            .unwrap();
    }

    storage
        .move_solutions_to_solved(&hashes[1..2])
        .await
        .unwrap();
    storage
        .move_solutions_to_failed(&[(hashes[2], SolutionFailReason::NotComposable)])
        .await
        .unwrap();

    let r = storage.list_blocks(None, None).await.unwrap();
    assert_eq!(r.len(), 3);

    // Get existing solution in solved
    let result = storage.get_solution(hashes[1]).await.unwrap().unwrap();
    assert_eq!(result.solution, solutions[1]);
    assert_eq!(result.outcome.len(), 3);
    assert_eq!(result.outcome[0], CheckOutcome::Success(0));
    assert_eq!(
        result.outcome[1],
        CheckOutcome::Fail(SolutionFailReason::NotComposable)
    );
    assert_eq!(result.outcome[2], CheckOutcome::Success(2));

    // Get existing solution in failed
    let result = storage.get_solution(hashes[2]).await.unwrap().unwrap();
    assert_eq!(result.solution, solutions[2]);
    assert_eq!(result.outcome.len(), 3);
    assert_eq!(
        result.outcome[0],
        CheckOutcome::Fail(SolutionFailReason::NotComposable)
    );
    assert_eq!(result.outcome[1], CheckOutcome::Success(1));
    assert_eq!(
        result.outcome[2],
        CheckOutcome::Fail(SolutionFailReason::NotComposable)
    );
}

create_test!(prune_failed_solutions);

async fn prune_failed_solutions<S: Storage>(storage: S) {
    let solutions: Vec<_> = (0..3).map(solution_with_all_inputs).collect();
    let hashes: Vec<_> = solutions.iter().map(essential_hash::hash).collect();

    for solution in &solutions {
        storage
            .insert_solution_into_pool(solution.clone())
            .await
            .unwrap();
    }

    for hash in &hashes {
        storage
            .move_solutions_to_failed(&[(*hash, SolutionFailReason::NotComposable)])
            .await
            .unwrap();
    }

    let result = storage.list_failed_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 3);

    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        + std::time::Duration::from_secs(100);

    storage.prune_failed_solutions(time).await.unwrap();

    let result = storage.list_failed_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 0);
}

create_test!(commit_block);

async fn commit_block<S: Storage>(storage: S) {
    let solutions: Vec<_> = (0..3).map(solution_with_all_inputs).collect();
    let hashes: Vec<_> = solutions.iter().map(essential_hash::hash).collect();

    for solution in &solutions {
        storage
            .insert_solution_into_pool(solution.clone())
            .await
            .unwrap();
    }

    let mut contract = sign_contract_with_random_keypair(vec![
        predicate_with_salt(0),
        predicate_with_salt(1),
        predicate_with_salt(2),
    ]);
    contract.contract.sort_by_key(essential_hash::content_addr);

    storage.insert_contract(contract.clone()).await.unwrap();

    let address = essential_hash::contract_addr::from_contract(&contract.contract);

    let failed = [(hashes[1], SolutionFailReason::NotComposable)];
    let solved = [hashes[2]];
    let state_updates = (0..10).map(|i| (address.clone(), vec![i as Word], vec![i as Word]));

    let data = CommitData {
        failed: &failed,
        solved: &solved,
        state_updates: Box::new(state_updates),
    };
    storage.commit_block(data).await.unwrap();

    let result = storage.get_solution(hashes[0]).await.unwrap().unwrap();
    assert_eq!(result.solution, solutions[0]);
    assert!(result.outcome.is_empty());

    let result = storage.get_solution(hashes[1]).await.unwrap().unwrap();
    assert_eq!(result.solution, solutions[1]);
    assert_eq!(result.outcome.len(), 1);
    assert_eq!(
        result.outcome[0],
        CheckOutcome::Fail(SolutionFailReason::NotComposable)
    );

    let result = storage.get_solution(hashes[2]).await.unwrap().unwrap();
    assert_eq!(result.solution, solutions[2]);
    assert_eq!(result.outcome.len(), 1);
    assert_eq!(result.outcome[0], CheckOutcome::Success(0));

    for i in 0..10 {
        let result = storage
            .query_state(&address, &vec![i as Word])
            .await
            .unwrap();
        assert_eq!(result, vec![i as Word]);
    }
}

create_test!(update_state);

async fn update_state<S: Storage>(storage: S) {
    let mut contract = sign_contract_with_random_keypair(vec![
        predicate_with_salt(0),
        predicate_with_salt(1),
        predicate_with_salt(2),
    ]);
    contract.contract.sort_by_key(essential_hash::content_addr);

    storage.insert_contract(contract.clone()).await.unwrap();

    let address = essential_hash::contract_addr::from_contract(&contract.contract);

    for i in 0..10 {
        let r = storage
            .query_state(&address, &vec![i as Word])
            .await
            .unwrap();
        assert!(r.is_empty());
        let r = storage
            .update_state(&address, &vec![i as Word], vec![i as Word])
            .await
            .unwrap();
        assert!(r.is_empty());
        let r = storage
            .update_state(&address, &vec![i as Word], vec![i as Word])
            .await
            .unwrap();
        assert_eq!(r, vec![i as Word]);
        let r = storage
            .query_state(&address, &vec![i as Word])
            .await
            .unwrap();
        assert_eq!(r, vec![i as Word]);
    }
}

create_test!(update_state_batch);

async fn update_state_batch<S: Storage>(storage: S) {
    let mut contract = sign_contract_with_random_keypair(vec![
        predicate_with_salt(0),
        predicate_with_salt(1),
        predicate_with_salt(2),
    ]);
    contract.contract.sort_by_key(essential_hash::content_addr);

    storage.insert_contract(contract.clone()).await.unwrap();

    let address = essential_hash::contract_addr::from_contract(&contract.contract);

    let updates = (0..10)
        .map(|i| (address.clone(), vec![i as Word], vec![i as Word]))
        .collect::<Vec<_>>();

    for (address, key, _) in &updates {
        let r = storage.query_state(address, key).await.unwrap();
        assert!(r.is_empty());
    }

    let r = storage.update_state_batch(updates.clone()).await.unwrap();
    assert_eq!(r.len(), 10);

    for r in r {
        assert!(r.is_empty());
    }

    let r = storage.update_state_batch(updates.clone()).await.unwrap();
    assert_eq!(r.len(), 10);

    for (i, r) in r.into_iter().enumerate() {
        assert_eq!(r, vec![i as Word]);
    }

    for (address, key, value) in &updates {
        let r = storage.query_state(address, key).await.unwrap();
        assert_eq!(r, value.clone());
    }
}
