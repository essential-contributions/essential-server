use std::{time::Duration, vec};

use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use essential_rest_server as server;
use essential_types::{
    convert::u8_32_from_word_4, intent::Intent, solution::Solution, Block, ContentAddress,
    IntentAddress, Signed, StorageLayout, Word,
};
use reqwest::Client;
use server::run;
use storage::{StateStorage, Storage};
use test_utils::{empty::Empty, sign_with_random_keypair};

static SERVER: &str = "localhost:0";
static CLIENT: &str = "http://localhost";

struct TestServer {
    client: Client,
    url: reqwest::Url,
    shutdown: tokio::sync::oneshot::Sender<()>,
    jh: tokio::task::JoinHandle<anyhow::Result<()>>,
}

async fn setup() -> TestServer {
    setup_with_mem(memory_storage::MemoryStorage::new()).await
}

async fn setup_with_mem(mem: memory_storage::MemoryStorage) -> TestServer {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let (shutdown, shutdown_rx) = tokio::sync::oneshot::channel();
    let jh = tokio::task::spawn(async {
        let essential = essential_server::Essential::new(mem);
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

    let intent_set = sign_with_random_keypair(vec![Intent::empty()]);
    let response = client
        .post(url.join("/deploy-intent-set").unwrap())
        .json(&intent_set)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let address = response.json::<ContentAddress>().await.unwrap();
    assert_eq!(address.0, utils::hash(&intent_set.data));

    let a = url
        .join(&format!("/get-intent-set/{}", URL_SAFE.encode(address.0)))
        .unwrap();
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let set = response
        .json::<Option<Signed<Vec<Intent>>>>()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(set, intent_set);

    let intent_address = IntentAddress {
        set: address,
        intent: ContentAddress(utils::hash(&intent_set.data[0])),
    };
    let a = url
        .join(&format!(
            "/get-intent/{}/{}",
            URL_SAFE.encode(intent_address.set.0),
            URL_SAFE.encode(intent_address.intent.0),
        ))
        .unwrap();
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let intent = response.json::<Option<Intent>>().await.unwrap().unwrap();

    assert_eq!(intent, intent_set.data[0]);

    let mut a = url.join("/list-intent-sets").unwrap();
    let time = std::time::UNIX_EPOCH.elapsed().unwrap() + Duration::from_secs(600);
    a.query_pairs_mut()
        .append_pair("start", "0")
        .append_pair("end", time.as_secs().to_string().as_str())
        .append_pair("page", "0");
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let intents = response.json::<Vec<Vec<Intent>>>().await.unwrap();
    assert_eq!(intents, vec![intent_set.data.clone()]);

    let a = url.join("/list-intent-sets").unwrap();
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let intents = response.json::<Vec<Vec<Intent>>>().await.unwrap();
    assert_eq!(intents, vec![intent_set.data.clone()]);

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
    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup().await;

    let solution = sign_with_random_keypair(Solution::empty());
    let response = client
        .post(url.join("/submit-solution").unwrap())
        .json(&solution)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let hash = response.json::<essential_types::Hash>().await.unwrap();
    assert_eq!(hash, utils::hash(&solution.data));

    let response = client
        .get(url.join("list-solutions-pool").unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let solutions = response.json::<Vec<Signed<Solution>>>().await.unwrap();

    assert_eq!(solutions.len(), 1);
    assert_eq!(utils::hash(&solutions[0].data), hash);

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}

#[tokio::test]
async fn test_query_state() {
    let intent_set = sign_with_random_keypair(vec![Intent::empty()]);
    let address = ContentAddress(utils::hash(&intent_set.data));
    let key = [0; 4];

    let mem = memory_storage::MemoryStorage::new();
    mem.insert_intent_set(StorageLayout {}, intent_set)
        .await
        .unwrap();
    mem.update_state(&address, &key, Some(42)).await.unwrap();

    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup_with_mem(mem).await;

    let a = url
        .join(&format!(
            "/query-state/{}/{}",
            URL_SAFE.encode(address.0),
            URL_SAFE.encode(u8_32_from_word_4(key)),
        ))
        .unwrap();
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let value = response.json::<Option<Word>>().await.unwrap().unwrap();

    assert_eq!(value, 42);

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}

#[tokio::test]
async fn test_list_winning_blocks() {
    let solution = sign_with_random_keypair(Solution::empty());
    let hash = utils::hash(&solution.data);

    let mem = memory_storage::MemoryStorage::new();
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
    assert_eq!(utils::hash(&blocks[0].batch.solutions[0].data), hash);

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}
