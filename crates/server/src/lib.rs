use std::{net::SocketAddr, time::Duration};

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use essential_server::{Essential, Storage};
use essential_types::{
    convert::word_4_from_u8_32, intent::Intent, solution::Solution, Block, ContentAddress, Hash,
    IntentAddress, Signed, Word,
};
use serde::Deserialize;
use tokio::{
    net::{TcpListener, ToSocketAddrs},
    sync::oneshot,
};

pub async fn run<S, A>(
    essential: Essential<S>,
    addr: A,
    local_addr: oneshot::Sender<SocketAddr>,
    shutdown_rx: Option<oneshot::Receiver<()>>,
) -> anyhow::Result<()>
where
    A: ToSocketAddrs,
    S: Storage + Clone + Send + Sync + 'static,
{
    let handle = essential.clone().spawn()?;
    let app = Router::new()
        .route("/deploy-intent-set", post(deploy_intent_set))
        .route("/get-intent-set/:address", get(get_intent_set))
        .route("/get-intent/:set/:address", get(get_intent))
        .route("/list-intent-sets", get(list_intent_sets))
        .route("/submit-solution", post(submit_solution))
        .route("/list-solutions-pool", get(list_solutions_pool))
        .route("/query-state/:address/:key", get(query_state))
        .route("/list-winning-blocks", get(list_winning_blocks))
        .with_state(essential.clone());
    let listener = TcpListener::bind(addr).await?;
    let addr = listener.local_addr()?;
    local_addr
        .send(addr)
        .map_err(|_| anyhow::anyhow!("Failed to send local address"))?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown(shutdown_rx))
        .await?;
    handle.shutdown().await?;
    Ok(())
}

async fn deploy_intent_set<S>(
    State(essential): State<Essential<S>>,
    Json(payload): Json<Signed<Vec<Intent>>>,
) -> Result<Json<ContentAddress>, Error>
where
    S: Storage + Clone + Send + Sync + 'static,
{
    let address = essential.deploy_intent_set(payload).await?;
    Ok(Json(address))
}

async fn get_intent_set<S>(
    State(essential): State<Essential<S>>,
    Path(address): Path<String>,
) -> Result<Json<Option<Signed<Vec<Intent>>>>, Error>
where
    S: Storage + Clone + Send + Sync + 'static,
{
    use base64::{engine::general_purpose::URL_SAFE, Engine as _};
    let address = ContentAddress(
        URL_SAFE
            .decode(address)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("Content Address wrong size"))?,
    );
    let set = essential.get_intent_set(&address).await?;
    Ok(Json(set))
}

async fn get_intent<S>(
    State(essential): State<Essential<S>>,
    Path((set, address)): Path<(String, String)>,
) -> Result<Json<Option<Intent>>, Error>
where
    S: Storage + Clone + Send + Sync + 'static,
{
    use base64::{engine::general_purpose::URL_SAFE, Engine as _};
    let set = ContentAddress(
        URL_SAFE
            .decode(set)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("Content Address wrong size"))?,
    );
    let intent = ContentAddress(
        URL_SAFE
            .decode(address)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("Content Address wrong size"))?,
    );
    let intent = essential.get_intent(&IntentAddress { set, intent }).await?;
    Ok(Json(intent))
}

#[derive(Deserialize)]
struct TimeRange {
    start: u64,
    end: u64,
}

#[derive(Deserialize)]
struct Page {
    page: u64,
}

async fn list_intent_sets<S>(
    State(essential): State<Essential<S>>,
    time_range: Option<Query<TimeRange>>,
    page: Option<Query<Page>>,
) -> Result<Json<Vec<Vec<Intent>>>, Error>
where
    S: Storage + Clone + Send + Sync + 'static,
{
    let time_range =
        time_range.map(|range| Duration::from_secs(range.start)..Duration::from_secs(range.end));

    let sets = essential
        .list_intent_sets(time_range, page.map(|p| p.page as usize))
        .await?;
    Ok(Json(sets))
}

async fn list_winning_blocks<S>(
    State(essential): State<Essential<S>>,
    time_range: Option<Query<TimeRange>>,
    page: Option<Query<Page>>,
) -> Result<Json<Vec<Block>>, Error>
where
    S: Storage + Clone + Send + Sync + 'static,
{
    let time_range =
        time_range.map(|range| Duration::from_secs(range.start)..Duration::from_secs(range.end));

    let blocks = essential
        .list_winning_blocks(time_range, page.map(|p| p.page as usize))
        .await?;
    Ok(Json(blocks))
}

async fn submit_solution<S>(
    State(essential): State<Essential<S>>,
    Json(payload): Json<Signed<Solution>>,
) -> Result<Json<Hash>, Error>
where
    S: Storage + Clone + Send + Sync + 'static,
{
    let hash = essential.submit_solution(payload).await?;
    Ok(Json(hash))
}

async fn list_solutions_pool<S>(
    State(essential): State<Essential<S>>,
) -> Result<Json<Vec<Signed<Solution>>>, Error>
where
    S: Storage + Clone + Send + Sync + 'static,
{
    let solutions = essential.list_solutions_pool().await?;
    Ok(Json(solutions))
}

async fn query_state<S>(
    State(essential): State<Essential<S>>,
    Path((address, key)): Path<(String, String)>,
) -> Result<Json<Option<Word>>, Error>
where
    S: Storage + Clone + Send + Sync + 'static,
{
    use base64::{engine::general_purpose::URL_SAFE, Engine as _};
    let address = ContentAddress(
        URL_SAFE
            .decode(address)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("Content Address wrong size"))?,
    );
    let key: [u8; 32] = URL_SAFE
        .decode(key)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("State key wrong size"))?;
    let key = word_4_from_u8_32(key);

    let state = essential.query_state(&address, &key).await?;
    Ok(Json(state))
}

async fn shutdown(rx: Option<oneshot::Receiver<()>>) {
    let manual = async {
        if let Some(rx) = rx {
            rx.await.ok();
        }
    };
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl-c");
    };

    tokio::select! {
        _ = manual => {},
        _ = ctrl_c => {},
    }
}

struct Error(anyhow::Error);

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("{}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for Error
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
