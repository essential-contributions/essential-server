use std::time::Duration;

use essential_server::{SolutionOutcome, StateRead, Storage};
use essential_types::{intent::Intent, IntentAddress};
use test_dbs::create_test;
use test_utils::{empty::Empty, sign_intent_set_with_random_keypair, solution_with_intent};

create_test!(solution_outcome);

async fn solution_outcome<S>(s: S)
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let intent_set = vec![Intent::empty()];
    let intent_address = essential_hash::content_addr(&intent_set[0]);
    let intent_set_addr = essential_hash::intent_set_addr::from_intents(&intent_set);
    let intent_address = IntentAddress {
        set: intent_set_addr,
        intent: intent_address,
    };

    let server = essential_server::Essential::new(s, Default::default());
    let config = essential_server::Config {
        run_loop_interval: Duration::from_millis(100),
    };
    let handle = server.clone().spawn(config).unwrap();

    let solution = solution_with_intent(intent_address);
    let solution_hash = essential_hash::hash(&solution);

    let intent_set = sign_intent_set_with_random_keypair(intent_set);

    server.deploy_intent_set(intent_set).await.unwrap();

    server.submit_solution(solution.clone()).await.unwrap();

    let blocks = loop {
        let blocks = server.list_winning_blocks(None, None).await.unwrap();
        if !blocks.is_empty() {
            break blocks;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    let outcome = server.solution_outcome(&solution_hash).await.unwrap();

    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].batch.solutions.len(), 1);
    assert!(&blocks[0].batch.solutions.contains(&solution));
    assert_eq!(outcome.len(), 1);
    assert_eq!(outcome[0], SolutionOutcome::Success(0));

    server.submit_solution(solution.clone()).await.unwrap();

    let blocks = loop {
        let blocks = server.list_winning_blocks(None, None).await.unwrap();
        if blocks.len() > 1 {
            break blocks;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    let outcome = server.solution_outcome(&solution_hash).await.unwrap();

    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[1].batch.solutions.len(), 1);
    assert!(&blocks[1].batch.solutions.contains(&solution));
    assert_eq!(outcome.len(), 2, "{:?}", outcome);
    assert_eq!(outcome[1], SolutionOutcome::Success(1));

    handle.shutdown().await.unwrap();
}
