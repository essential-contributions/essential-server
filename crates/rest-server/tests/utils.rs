#![allow(dead_code)]

use std::sync::Arc;

use essential_memory_storage::MemoryStorage;
use essential_rest_server::run;
use essential_server::TimeConfig;
use reqwest::{Client, ClientBuilder};

static SERVER: &str = "localhost:0";
static CLIENT: &str = "http://localhost";

pub struct TestServer {
    pub client: Client,
    pub url: reqwest::Url,
    pub shutdown: tokio::sync::oneshot::Sender<()>,
    pub jh: tokio::task::JoinHandle<anyhow::Result<()>>,
}

pub async fn setup() -> TestServer {
    setup_with_mem(MemoryStorage::new()).await
}

pub async fn setup_with_mem(mem: MemoryStorage) -> TestServer {
    let config = Default::default();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let (shutdown, shutdown_rx) = tokio::sync::oneshot::channel();
    let jh = tokio::task::spawn(async {
        let essential = essential_server::Essential::new(
            mem,
            config,
            Arc::new(TimeConfig {
                enable_time: false,
                ..Default::default()
            }),
        );
        run(essential, SERVER, tx, Some(shutdown_rx), Default::default()).await
    });
    let client = ClientBuilder::new()
        .http2_prior_knowledge()
        .build()
        .unwrap();
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
