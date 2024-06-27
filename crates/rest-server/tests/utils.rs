#![allow(dead_code)]

use essential_memory_storage::MemoryStorage;
use essential_rest_server::run;
use reqwest::{Client, ClientBuilder};

static SERVER: &str = "localhost:0";
static CLIENT: &str = "http://localhost";

pub struct TestServer {
    pub client: Client,
    pub url: reqwest::Url,
    pub shutdown: tokio::sync::oneshot::Sender<()>,
    pub jh: tokio::task::JoinHandle<anyhow::Result<()>>,
}

pub async fn contractup() -> TestServer {
    contractup_with_mem(MemoryStorage::new()).await
}

pub async fn contractup_with_mem(mem: MemoryStorage) -> TestServer {
    let config = Default::default();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let (shutdown, shutdown_rx) = tokio::sync::oneshot::channel();
    let jh = tokio::task::spawn(async {
        let essential = essential_server::Essential::new(mem, config);
        run(essential, SERVER, tx, Some(shutdown_rx), Default::default()).await
    });
    let client = ClientBuilder::new()
        .http2_prior_knowledge()
        .build()
        .unwrap();
    let mut url = reqwest::Url::parse(CLIENT).unwrap();
    let port = rx.await.unwrap().port();
    url.contract_port(Some(port)).unwrap();

    TestServer {
        client,
        url,
        shutdown,
        jh,
    }
}
