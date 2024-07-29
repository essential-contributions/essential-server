use std::time::Duration;

use essential_hash::hash;
use essential_storage::{
    failed_solution::{CheckOutcome, SolutionFailReason},
    Storage,
};
use essential_types::PredicateAddress;
use futures::{StreamExt, TryStreamExt};
use test_dbs::create_test;
use test_utils::{
    predicate_with_salt, sign_contract_with_random_keypair, solution_with_all_inputs_fixed_size,
    solution_with_decision_variables,
};

create_test!(insert_contract);

async fn insert_contract<S: Storage>(storage: S) {
    let mut contracts = [
        sign_contract_with_random_keypair(vec![
            predicate_with_salt(0),
            predicate_with_salt(1),
            predicate_with_salt(2),
        ]),
        sign_contract_with_random_keypair(vec![
            predicate_with_salt(2),
            predicate_with_salt(3),
            predicate_with_salt(4),
        ]),
    ];

    // Order contract by their CA, as that's how `list_contracts` will return them.
    for signed in &mut contracts {
        signed.contract.sort_by_key(essential_hash::content_addr);
    }

    storage.insert_contract(contracts[0].clone()).await.unwrap();

    storage.insert_contract(contracts[0].clone()).await.unwrap();

    let result = storage.list_contracts(None, None).await.unwrap();
    assert_eq!(result, vec![contracts[0].contract.clone()]);

    storage.insert_contract(contracts[1].clone()).await.unwrap();

    let result = storage.list_contracts(None, None).await.unwrap();
    assert_eq!(
        result,
        vec![contracts[0].contract.clone(), contracts[1].contract.clone()]
    );

    for contract in &contracts {
        for predicate in &contract.contract.predicates {
            let address = PredicateAddress {
                contract: essential_hash::contract_addr::from_contract(&contract.contract),
                predicate: essential_hash::content_addr(predicate),
            };
            let result = storage.get_predicate(&address).await.unwrap().unwrap();
            assert_eq!(&result, predicate);
        }
    }
}

create_test!(subscribe_contracts);

async fn subscribe_contracts<S: Storage + Clone + Send + Sync + 'static>(storage: S) {
    let mut contracts = [
        sign_contract_with_random_keypair(vec![
            predicate_with_salt(0),
            predicate_with_salt(1),
            predicate_with_salt(2),
        ]),
        sign_contract_with_random_keypair(vec![
            predicate_with_salt(2),
            predicate_with_salt(3),
            predicate_with_salt(4),
        ]),
    ];

    // Order contract by their CA, as that's how `list_contracts` will return them.
    for signed in &mut contracts {
        signed.contract.sort_by_key(essential_hash::content_addr);
    }

    storage.insert_contract(contracts[0].clone()).await.unwrap();

    let (tx, rx) = tokio::sync::oneshot::channel();

    let jh = tokio::spawn({
        let storage = storage.clone();
        let contracts = contracts.clone();
        async move {
            rx.await.unwrap();
            storage.insert_contract(contracts[1].clone()).await.unwrap();
        }
    });

    let stream = storage.subscribe_contracts(None, None);
    futures::pin_mut!(stream);
    let result = stream.next().await.unwrap().unwrap();
    assert_eq!(result, contracts[0].contract);

    let r = tokio::time::timeout(Duration::from_millis(50), stream.next()).await;
    assert!(r.is_err());
    tx.send(()).unwrap();

    let result = stream.next().await.unwrap().unwrap();
    assert_eq!(result, contracts[1].contract);
    jh.await.unwrap();
}

create_test!(solutions);

async fn solutions<S: Storage>(storage: S) {
    let solution = solution_with_decision_variables(0);
    let solution2 = solution_with_decision_variables(1);
    let solution3 = solution_with_decision_variables(2);
    let solution4 = solution_with_decision_variables(3);

    // Idempotent insert
    storage
        .insert_solution_into_pool(solution.clone())
        .await
        .unwrap();
    storage
        .insert_solution_into_pool(solution.clone())
        .await
        .unwrap();

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result, vec![solution.clone()]);

    storage
        .insert_solution_into_pool(solution2.clone())
        .await
        .unwrap();
    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 2);
    assert!(result.contains(&solution));
    assert!(result.contains(&solution2));

    storage
        .move_solutions_to_solved(&[hash(&solution)])
        .await
        .unwrap();

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 1);
    assert!(result.contains(&solution2));

    let result = storage.list_blocks(None, None, None).await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].solutions, vec![solution.clone()]);

    storage
        .insert_solution_into_pool(solution3.clone())
        .await
        .unwrap();

    storage
        .insert_solution_into_pool(solution4.clone())
        .await
        .unwrap();

    storage
        .move_solutions_to_solved(&[hash(&solution2), hash(&solution3)])
        .await
        .unwrap();

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 1);
    assert!(result.contains(&solution4));

    let solution4_hash = hash(&solution4);
    let solution4_fail_reason = SolutionFailReason::NotComposable;
    storage
        .move_solutions_to_failed(&[(solution4_hash, solution4_fail_reason.clone())])
        .await
        .unwrap();

    let result = storage.list_failed_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].solution, solution4);

    let result = storage.get_solution(solution4_hash).await.unwrap().unwrap();
    assert_eq!(
        result.outcome,
        vec![CheckOutcome::Fail(solution4_fail_reason)]
    );

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert!(result.is_empty());

    let result = storage.list_blocks(None, None, None).await.unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].solutions, vec![solution.clone()]);
    assert_eq!(
        result[1].solutions,
        vec![solution2.clone(), solution3.clone()]
    );

    storage
        .prune_failed_solutions(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                + std::time::Duration::from_secs(10),
        )
        .await
        .unwrap();

    let result = storage.list_failed_solutions_pool(None).await.unwrap();
    assert!(result.is_empty());
}

create_test!(subscribe_blocks);

async fn subscribe_blocks<S: Storage + Clone + Send + Sync + 'static>(storage: S) {
    let solutions: Vec<_> = (0..102)
        .map(|i| solution_with_all_inputs_fixed_size(i, 1))
        .collect();

    let hashes: Vec<_> = solutions.iter().map(essential_hash::hash).collect();

    for solution in &solutions {
        storage
            .insert_solution_into_pool(solution.clone())
            .await
            .unwrap();
    }

    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<_>>(10);

    let jh = tokio::spawn({
        let storage = storage.clone();
        async move {
            while let Some(hashes) = rx.recv().await {
                storage.move_solutions_to_solved(&hashes).await.unwrap();
            }
        }
    });

    tx.send(hashes[0..1].to_vec()).await.unwrap();

    let stream = storage.clone().subscribe_blocks(None, None, None);
    futures::pin_mut!(stream);
    let result = stream.next().await.unwrap().unwrap();
    assert_eq!(result.solutions[0], solutions[0]);

    let r = tokio::time::timeout(Duration::from_millis(50), stream.next()).await;
    assert!(r.is_err());
    tx.send(hashes[1..2].to_vec()).await.unwrap();

    let result = stream.next().await.unwrap().unwrap();
    assert_eq!(result.solutions[0], solutions[1]);

    // Move remaining solutions to solved in individual blocks
    for i in 2..102 {
        tx.send(hashes[i..i + 1].to_vec()).await.unwrap();
        let result = stream.next().await.unwrap().unwrap();
        assert_eq!(result.solutions[0], solutions[i]);
    }

    let results: Vec<_> = storage
        .clone()
        .subscribe_blocks(None, None, Some(1))
        .take_until(tokio::time::sleep(Duration::from_millis(50)))
        .try_collect()
        .await
        .unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].solutions[0], solutions[100]);
    assert_eq!(results[1].solutions[0], solutions[101]);

    let results: Vec<_> = storage
        .clone()
        .subscribe_blocks(None, Some(99), Some(0))
        .take_until(tokio::time::sleep(Duration::from_millis(50)))
        .try_collect()
        .await
        .unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].solutions[0], solutions[99]);
    assert_eq!(results[1].solutions[0], solutions[100]);
    assert_eq!(results[2].solutions[0], solutions[101]);

    // List block num 1 and page 1
    let results: Vec<_> = storage
        .clone()
        .subscribe_blocks(None, Some(1), Some(1))
        .take_until(tokio::time::sleep(Duration::from_millis(50)))
        .try_collect()
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].solutions[0], solutions[101]);

    // List block num 101
    let results: Vec<_> = storage
        .clone()
        .subscribe_blocks(None, Some(101), Some(0))
        .take_until(tokio::time::sleep(Duration::from_millis(50)))
        .try_collect()
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].solutions[0], solutions[101]);

    // List empty block num
    let results: Vec<_> = storage
        .clone()
        .subscribe_blocks(None, Some(200), None)
        .take_until(tokio::time::sleep(Duration::from_millis(50)))
        .try_collect()
        .await
        .unwrap();
    assert!(results.is_empty());

    // List empty page
    let results: Vec<_> = storage
        .clone()
        .subscribe_blocks(None, None, Some(2))
        .take_until(tokio::time::sleep(Duration::from_millis(50)))
        .try_collect()
        .await
        .unwrap();
    assert!(results.is_empty());

    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let start = time - std::time::Duration::from_secs(100);

    // List within start time
    let results: Vec<_> = storage
        .clone()
        .subscribe_blocks(Some(start), None, None)
        .take_until(tokio::time::sleep(Duration::from_millis(50)))
        .try_collect()
        .await
        .unwrap();

    assert_eq!(results.len(), 102);
    for (i, result) in results.iter().enumerate() {
        assert_eq!(result.solutions[0], solutions[i]);
    }

    // List within time and block num
    let results: Vec<_> = storage
        .clone()
        .subscribe_blocks(Some(start), Some(2), None)
        .take_until(tokio::time::sleep(Duration::from_millis(50)))
        .try_collect()
        .await
        .unwrap();

    assert_eq!(results.len(), 100);
    for (i, result) in results.iter().enumerate() {
        assert_eq!(result.solutions[0], solutions[i + 2]);
    }

    // List within time, block, page
    let results: Vec<_> = storage
        .clone()
        .subscribe_blocks(Some(start), Some(1), Some(1))
        .take_until(tokio::time::sleep(Duration::from_millis(50)))
        .try_collect()
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].solutions[0], solutions[101]);

    // List outside time
    let start = start + std::time::Duration::from_secs(200);
    let results: Vec<_> = storage
        .clone()
        .subscribe_blocks(Some(start), None, None)
        .take_until(tokio::time::sleep(Duration::from_millis(50)))
        .try_collect()
        .await
        .unwrap();
    assert!(results.is_empty());

    drop(tx);

    jh.await.unwrap();
}

create_test!(update_and_query_state);

async fn update_and_query_state<S: Storage>(storage: S) {
    let contract = sign_contract_with_random_keypair(vec![predicate_with_salt(0)]);
    let address = essential_hash::contract_addr::from_contract(&contract.contract);
    let key = vec![0; 4];
    let word = vec![42];

    // Test updating the state without an contract
    storage
        .update_state(&address, &key, word.clone())
        .await
        .unwrap_err();

    // Test querying the state
    let query_result = storage.query_state(&address, &key).await.unwrap();
    assert!(query_result.is_empty());

    storage.insert_contract(contract.clone()).await.unwrap();

    // Test updating the state
    let old = storage
        .update_state(&address, &key, word.clone())
        .await
        .unwrap();
    assert!(old.is_empty());

    // Test querying the state
    let query_result = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(query_result, word);

    // Test updating the state
    let old = storage.update_state(&address, &key, vec![1]).await.unwrap();
    assert_eq!(old, word);

    // Test querying the state
    let query_result = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(query_result, vec![1]);

    // Test querying empty state
    let query_result = storage.query_state(&address, &vec![1; 4]).await.unwrap();
    assert!(query_result.is_empty());
}

create_test!(double_get_solution_bug);

async fn double_get_solution_bug<S: Storage>(storage: S) {
    let solution = solution_with_decision_variables(0);
    let hash = hash(&solution);

    storage
        .insert_solution_into_pool(solution.clone())
        .await
        .unwrap();

    storage
        .insert_solution_into_pool(solution.clone())
        .await
        .unwrap();

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 1);

    storage.move_solutions_to_solved(&[hash]).await.unwrap();

    let result = storage.get_solution(hash).await.unwrap().unwrap();
    assert_eq!(result.outcome, vec![CheckOutcome::Success(0)]);

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert!(result.is_empty());

    storage
        .insert_solution_into_pool(solution.clone())
        .await
        .unwrap();

    storage
        .insert_solution_into_pool(solution.clone())
        .await
        .unwrap();

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 1);

    let result = storage.get_solution(hash).await.unwrap().unwrap();
    assert_eq!(result.outcome, vec![CheckOutcome::Success(0)]);

    storage.move_solutions_to_solved(&[hash]).await.unwrap();

    let result = storage.get_solution(hash).await.unwrap().unwrap();
    assert_eq!(
        result.outcome,
        vec![CheckOutcome::Success(0), CheckOutcome::Success(1)]
    );

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert!(result.is_empty());
}

create_test!(list_solutions_pool_order);

async fn list_solutions_pool_order<S: Storage>(storage: S) {
    let solutions = (0..10)
        .map(solution_with_decision_variables)
        .collect::<Vec<_>>();

    let hashes = solutions.iter().map(hash).collect::<Vec<_>>();

    for solution in &solutions {
        storage
            .insert_solution_into_pool(solution.clone())
            .await
            .unwrap();
    }

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result, solutions);

    storage.move_solutions_to_solved(&hashes).await.unwrap();

    for solution in solutions.iter().rev() {
        storage
            .insert_solution_into_pool(solution.clone())
            .await
            .unwrap();
    }

    let result = storage.list_solutions_pool(None).await.unwrap();

    let mut expected = solutions.clone();
    expected.reverse();
    assert_eq!(result, expected);
}
