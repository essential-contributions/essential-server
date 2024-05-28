use std::{time::Duration, vec};

use base64::Engine as _;
use essential_memory_storage::MemoryStorage;
use essential_rest_server as server;
use essential_server::{CheckSolutionOutput, SolutionOutcome};
use essential_storage::{StateStorage, Storage};
use essential_types::{
    convert::bytes_from_word,
    intent::{self, Intent},
    solution::{Solution, SolutionData},
    Block, ContentAddress, IntentAddress, StorageLayout, Word,
};
use reqwest::Client;
use server::run;
use test_utils::{
    empty::Empty, sign_intent_set_with_random_keypair, solution_with_decision_variables,
};

static SERVER: &str = "localhost:0";
static CLIENT: &str = "http://localhost";

struct TestServer {
    client: Client,
    url: reqwest::Url,
    shutdown: tokio::sync::oneshot::Sender<()>,
    jh: tokio::task::JoinHandle<anyhow::Result<()>>,
}

async fn setup() -> TestServer {
    setup_with_mem(MemoryStorage::new()).await
}

async fn setup_with_mem(mem: MemoryStorage) -> TestServer {
    let config = Default::default();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let (shutdown, shutdown_rx) = tokio::sync::oneshot::channel();
    let jh = tokio::task::spawn(async {
        let essential = essential_server::Essential::new(mem, config);
        run(essential, SERVER, tx, Some(shutdown_rx)).await
    });
    let client = Client::new();
    let mut url = reqwest::Url::parse(CLIENT).unwrap();
    let port = rx.await.unwrap().port();
    url.set_port(Some(port)).unwrap();

    TestServer {
        client,
        url,
        shutdown,
        jh,
    }
}

#[tokio::test]
async fn test_deploy_intent_set() {
    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup().await;

    let intent_set = sign_intent_set_with_random_keypair(vec![Intent::empty()]);
    let response = client
        .post(url.join("/deploy-intent-set").unwrap())
        .json(&intent_set)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let address = response.json::<ContentAddress>().await.unwrap();
    let expected = essential_hash::intent_set_addr::from_intents(&intent_set.set);
    assert_eq!(address, expected);

    let a = url.join(&format!("/get-intent-set/{address}")).unwrap();
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let set = response
        .json::<Option<intent::SignedSet>>()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(set, intent_set);

    let intent_address = IntentAddress {
        set: address,
        intent: essential_hash::content_addr(&intent_set.set[0]),
    };
    let a = url
        .join(&format!(
            "/get-intent/{}/{}",
            intent_address.set, intent_address.intent,
        ))
        .unwrap();
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let intent = response.json::<Option<Intent>>().await.unwrap().unwrap();

    assert_eq!(intent, intent_set.set[0]);

    let mut a = url.join("/list-intent-sets").unwrap();
    let time = std::time::UNIX_EPOCH.elapsed().unwrap() + Duration::from_secs(600);
    a.query_pairs_mut()
        .append_pair("start", "0")
        .append_pair("end", time.as_secs().to_string().as_str())
        .append_pair("page", "0");
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let intents = response.json::<Vec<Vec<Intent>>>().await.unwrap();
    assert_eq!(intents, vec![intent_set.set.clone()]);

    let a = url.join("/list-intent-sets").unwrap();
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let intents = response.json::<Vec<Vec<Intent>>>().await.unwrap();
    assert_eq!(intents, vec![intent_set.set.clone()]);

    let mut a = url.join("/list-intent-sets").unwrap();
    a.query_pairs_mut().append_pair("page", "1");
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let intents = response.json::<Vec<Vec<Intent>>>().await.unwrap();
    assert!(intents.is_empty());

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}

#[tokio::test]
async fn test_submit_solution() {
    let intent = Intent::empty();
    let intent_addr = essential_hash::content_addr(&intent);
    let intent_set = sign_intent_set_with_random_keypair(vec![intent]);
    let set_addr = essential_hash::intent_set_addr::from_intents(&intent_set.set);

    let mem = MemoryStorage::new();
    mem.insert_intent_set(StorageLayout {}, intent_set)
        .await
        .unwrap();

    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup_with_mem(mem).await;
    let mut solution = solution_with_decision_variables(1);
    solution.data[0].intent_to_solve = IntentAddress {
        set: set_addr,
        intent: intent_addr,
    };
    let response = client
        .post(url.join("/submit-solution").unwrap())
        .json(&solution)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let ca = response
        .json::<essential_types::ContentAddress>()
        .await
        .unwrap();
    assert_eq!(ca, essential_hash::content_addr(&solution));

    let response = client
        .get(url.join("list-solutions-pool").unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let solutions = response.json::<Vec<Solution>>().await.unwrap();

    assert_eq!(solutions.len(), 1);
    assert_eq!(essential_hash::content_addr(&solutions[0]), ca);

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}

#[tokio::test]
async fn test_query_state() {
    let intent_set = sign_intent_set_with_random_keypair(vec![Intent::empty()]);
    let address = essential_hash::intent_set_addr::from_intents(&intent_set.set);
    let key = vec![0; 4];

    let mem = MemoryStorage::new();
    mem.insert_intent_set(StorageLayout {}, intent_set)
        .await
        .unwrap();
    mem.update_state(&address, &key, vec![42]).await.unwrap();

    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup_with_mem(mem).await;

    let a = url
        .join(&format!(
            "/query-state/{address}/{}",
            essential_types::serde::hash::BASE64.encode(
                key.into_iter()
                    .flat_map(bytes_from_word)
                    .collect::<Vec<u8>>()
            ),
        ))
        .unwrap();
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let value = response.json::<Vec<Word>>().await.unwrap();

    assert_eq!(value, vec![42]);

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}

#[tokio::test]
async fn test_list_winning_blocks() {
    let solution = Solution::empty();
    let hash = essential_hash::hash(&solution);

    let mem = MemoryStorage::new();
    mem.insert_solution_into_pool(solution).await.unwrap();
    mem.move_solutions_to_solved(&[hash]).await.unwrap();

    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup_with_mem(mem).await;

    let mut a = url.join("/list-winning-blocks").unwrap();
    a.query_pairs_mut().append_pair("page", "0");
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let blocks = response.json::<Vec<Block>>().await.unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(essential_hash::hash(&blocks[0].batch.solutions[0]), hash);

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}

#[tokio::test]
async fn test_solution_outcome() {
    let solution = Solution::empty();
    let ca = essential_hash::content_addr(&solution);

    let mem = MemoryStorage::new();
    mem.insert_solution_into_pool(solution).await.unwrap();
    mem.move_solutions_to_solved(&[ca.0]).await.unwrap();

    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup_with_mem(mem).await;

    let a = url.join(&format!("/solution-outcome/{ca}")).unwrap();
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let value = response
        .json::<Option<SolutionOutcome>>()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(value, SolutionOutcome::Success(0));

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}

#[tokio::test]
async fn test_check_solution() {
    let intent_set = sign_intent_set_with_random_keypair(vec![Intent::empty()]);
    let mem = MemoryStorage::new();
    mem.insert_intent_set(StorageLayout {}, intent_set.clone())
        .await
        .unwrap();

    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup_with_mem(mem).await;

    let set = essential_hash::intent_set_addr::from_intents(&intent_set.set);
    let address = essential_hash::content_addr(&intent_set.set[0]);
    let mut solution = Solution::empty();
    solution.data.push(SolutionData {
        intent_to_solve: IntentAddress {
            set,
            intent: address,
        },
        decision_variables: vec![],
    });
    let response = client
        .post(url.join("/check-solution").unwrap())
        .json(&solution)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let value = response
        .json::<Option<CheckSolutionOutput>>()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        value,
        CheckSolutionOutput {
            utility: 1.0,
            gas: 0
        }
    );

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}

#[tokio::test]
async fn test_check_solution_with_data() {
    #[derive(serde::Serialize)]
    struct CheckSolution {
        solution: Solution,
        intents: Vec<Intent>,
    }
    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup().await;

    let intent_set = vec![Intent::empty()];
    let set = ContentAddress(essential_hash::hash(&intent_set));
    let address = ContentAddress(essential_hash::hash(&intent_set[0]));
    let mut solution = Solution::empty();
    solution.data.push(SolutionData {
        intent_to_solve: IntentAddress {
            set,
            intent: address,
        },
        decision_variables: vec![],
    });
    let input = CheckSolution {
        solution,
        intents: intent_set,
    };
    let response = client
        .post(url.join("/check-solution-with-data").unwrap())
        .json(&input)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let value = response
        .json::<Option<CheckSolutionOutput>>()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        value,
        CheckSolutionOutput {
            utility: 1.0,
            gas: 0
        }
    );

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}
