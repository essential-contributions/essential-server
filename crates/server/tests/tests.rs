use std::time::Duration;

use essential_server::{SolutionOutcome, StateRead, Storage};
use essential_types::{predicate::Predicate, PredicateAddress};
use test_dbs::create_test;
use test_utils::{empty::Empty, sign_contract_with_random_keypair, solution_with_predicate};

create_test!(solution_outcome);

async fn solution_outcome<S>(s: S)
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let contract = vec![Predicate::empty()];
    let predicate_address = essential_hash::content_addr(&contract[0]);
    let contract_addr = essential_hash::contract_addr::from_contract(&contract.clone().into());
    let predicate_address = PredicateAddress {
        contract: contract_addr,
        predicate: predicate_address,
    };

    let server = essential_server::Essential::new(s, Default::default(), Default::default());
    let config = essential_server::Config {
        run_loop_interval: Duration::from_millis(100),
    };
    let handle = server.clone().spawn(config).unwrap();

    let solution = solution_with_predicate(predicate_address);
    let solution_hash = essential_hash::hash(&solution);

    let contract = sign_contract_with_random_keypair(contract);

    server.deploy_contract(contract).await.unwrap();

    server.submit_solution(solution.clone()).await.unwrap();

    let blocks = loop {
        let blocks = server.list_blocks(None, None, None).await.unwrap();
        if !blocks.is_empty() {
            break blocks;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    let outcome = server.solution_outcome(&solution_hash).await.unwrap();

    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].solutions.len(), 1);
    assert!(&blocks[0].solutions.contains(&solution));
    assert_eq!(outcome.len(), 1);
    assert_eq!(outcome[0], SolutionOutcome::Success(0));

    server.submit_solution(solution.clone()).await.unwrap();

    let blocks = loop {
        let blocks = server.list_blocks(None, None, None).await.unwrap();
        if blocks.len() > 1 {
            break blocks;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    let outcome = server.solution_outcome(&solution_hash).await.unwrap();

    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[1].solutions.len(), 1);
    assert!(&blocks[1].solutions.contains(&solution));
    assert_eq!(outcome.len(), 2, "{:?}", outcome);
    assert_eq!(outcome[1], SolutionOutcome::Success(1));

    handle.shutdown().await.unwrap();
}
