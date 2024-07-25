use essential_hash::hash;
use essential_storage::Storage;
use essential_types::{predicate::Predicate, solution::Solution, PredicateAddress};
use std::vec;
use test_dbs::create_test;
use test_utils::{empty::Empty, predicate_with_salt, sign_contract_with_random_keypair};

create_test!(update_state);

async fn update_state<S: Storage>(storage: S) {
    let predicate = sign_contract_with_random_keypair(vec![Predicate::empty()]);
    storage.insert_contract(predicate).await.unwrap();
    let address = essential_hash::contract_addr::from_contract(&vec![Predicate::empty()].into());
    let key = vec![0; 4];
    let v = storage.update_state(&address, &key, vec![1]).await.unwrap();
    assert!(v.is_empty());
    let v = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(v, vec![1]);
    let v = storage.update_state(&address, &key, vec![2]).await.unwrap();
    assert_eq!(v, vec![1]);
    let v = storage.update_state(&address, &key, vec![]).await.unwrap();
    assert_eq!(v, vec![2]);
    let v = storage.update_state(&address, &key, vec![]).await.unwrap();
    assert!(v.is_empty());
    let v = storage.update_state(&address, &key, vec![1]).await.unwrap();
    assert!(v.is_empty());
    let v = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(v, vec![1]);

    let v = storage.query_state(&address, &vec![1; 14]).await.unwrap();
    assert!(v.is_empty());
    let v = storage
        .update_state(&address, &vec![1; 14], vec![3; 8])
        .await
        .unwrap();
    assert!(v.is_empty());
    let v = storage.query_state(&address, &vec![1; 14]).await.unwrap();
    assert_eq!(v, vec![3; 8]);
    let v = storage
        .update_state(&address, &vec![1; 14], vec![3; 2])
        .await
        .unwrap();
    assert_eq!(v, vec![3; 8]);
    let v = storage.query_state(&address, &vec![1; 14]).await.unwrap();
    assert_eq!(v, vec![3; 2]);
}

create_test!(update_state_batch);

async fn update_state_batch<S: Storage>(storage: S) {
    let predicate = sign_contract_with_random_keypair(vec![Predicate::empty()]);
    storage.insert_contract(predicate).await.unwrap();
    let predicate = sign_contract_with_random_keypair(vec![predicate_with_salt(3)]);
    storage.insert_contract(predicate).await.unwrap();
    let address_0 = essential_hash::contract_addr::from_contract(&vec![Predicate::empty()].into());
    let address_1 =
        essential_hash::contract_addr::from_contract(&vec![predicate_with_salt(3)].into());
    let key = vec![0; 4];
    let v = storage
        .update_state(&address_0, &key, vec![1])
        .await
        .unwrap();
    assert!(v.is_empty());
    let v = storage
        .update_state(&address_1, &vec![1; 4], vec![2])
        .await
        .unwrap();
    assert!(v.is_empty());
    let updates = (0..10).map(|i| {
        let address = if i % 2 == 0 {
            address_0.clone()
        } else {
            address_1.clone()
        };
        (address, vec![i; 4], vec![i])
    });
    let v = storage.update_state_batch(updates).await.unwrap();
    assert_eq!(
        v,
        vec![
            vec![1],
            vec![2],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![]
        ]
    );

    let v = storage.query_state(&address_0, &vec![8; 4]).await.unwrap();
    assert_eq!(v, vec![8]);
}

create_test!(insert_contract);

async fn insert_contract<S: Storage>(storage: S) {
    let predicate_0 = sign_contract_with_random_keypair(vec![Predicate::empty()]);
    storage.insert_contract(predicate_0.clone()).await.unwrap();
    let predicate_1 =
        sign_contract_with_random_keypair(vec![predicate_with_salt(1), predicate_with_salt(2)]);
    storage.insert_contract(predicate_1).await.unwrap();
    let contracts = storage.list_contracts(None, None).await.unwrap();
    let mut s = vec![predicate_with_salt(1), predicate_with_salt(2)];
    s.sort_by_key(essential_hash::content_addr);
    assert_eq!(contracts, vec![vec![Predicate::empty()].into(), s.into()]);
    let address = essential_hash::contract_addr::from_contract(&vec![Predicate::empty()].into());
    let contract = storage.get_contract(&address).await.unwrap();
    assert_eq!(contract, Some(predicate_0));

    let address = PredicateAddress {
        contract: essential_hash::contract_addr::from_contract(&vec![Predicate::empty()].into()),
        predicate: essential_hash::content_addr(&Predicate::empty()),
    };
    let predicate = storage.get_predicate(&address).await.unwrap();

    assert_eq!(predicate, Some(Predicate::empty()));
}

create_test!(insert_solution_into_pool);

async fn insert_solution_into_pool<S: Storage>(storage: S) {
    let solution = Solution::empty();
    storage
        .insert_solution_into_pool(solution.clone())
        .await
        .unwrap();
    let solutions = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(solutions.len(), 1);
    assert_eq!(hash(&solutions[0].data), hash(&Solution::empty()));
    storage
        .move_solutions_to_solved(&[hash(&Solution::empty())])
        .await
        .unwrap();
    let solutions = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(solutions.len(), 0);
    let batches = storage.list_blocks(None, None, None).await.unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(hash(&batches[0].solutions), hash(&vec![solution]));
}
