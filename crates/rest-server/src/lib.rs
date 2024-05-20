#![deny(missing_docs)]
//! # Server
//!
//! A simple REST server for the Essential platform.

use std::{net::SocketAddr, time::Duration};

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use essential_server::{CheckSolutionOutput, Essential, SolutionOutcome, StateRead, Storage};
use essential_types::{
    convert::word_4_from_u8_32, intent::Intent, solution::Solution, Block, ContentAddress, Hash,
    IntentAddress, Signed, Word,
};
use serde::Deserialize;
use tokio::{
    net::{TcpListener, ToSocketAddrs},
    sync::oneshot,
};

#[derive(Deserialize)]
/// Type to deserialize a time range query parameters.
struct TimeRange {
    /// Start of the time range in seconds.
    start: u64,
    /// End of the time range in seconds.
    end: u64,
}

#[derive(Deserialize)]
/// Type to deserialize a page query parameter.
struct Page {
    /// The page number to start from.
    page: u64,
}

#[derive(Deserialize)]
struct CheckSolution {
    solution: Signed<Solution>,
    intents: Vec<Intent>,
}

/// Run the server.
///
/// - Takes the essential library to run it.
/// - Address to bind to.
/// - A channel that returns the actual chosen local address.
/// - An optional channel that can be used to shutdown the server.
pub async fn run<S, A>(
    essential: Essential<S>,
    addr: A,
    local_addr: oneshot::Sender<SocketAddr>,
    shutdown_rx: Option<oneshot::Receiver<()>>,
) -> anyhow::Result<()>
where
    A: ToSocketAddrs,
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    // Spawn essential and get the handle.
    let handle = essential.clone().spawn()?;

    // Create all the endpoints.
    let app = Router::new()
        .route("/deploy-intent-set", post(deploy_intent_set))
        .route("/get-intent-set/:address", get(get_intent_set))
        .route("/get-intent/:set/:address", get(get_intent))
        .route("/list-intent-sets", get(list_intent_sets))
        .route("/submit-solution", post(submit_solution))
        .route("/list-solutions-pool", get(list_solutions_pool))
        .route("/query-state/:address/:key", get(query_state))
        .route("/list-winning-blocks", get(list_winning_blocks))
        .route("/solution-outcome/:hash", get(solution_outcome))
        .route("/check-solution", post(check_solution))
        .route("/check-solution-with-data", post(check_solution_with_data))
        .with_state(essential.clone());

    // Bind to the address.
    let listener = TcpListener::bind(addr).await?;

    // Send the local address to the caller.
    // This is useful when the address or port is chosen by the OS.
    let addr = listener.local_addr()?;
    local_addr
        .send(addr)
        .map_err(|_| anyhow::anyhow!("Failed to send local address"))?;

    // Serve the app.
    axum::serve(listener, app)
        // Attach the shutdown signal.
        .with_graceful_shutdown(shutdown(shutdown_rx))
        .await?;

    // After the server is done, shutdown essential.
    handle.shutdown().await?;

    Ok(())
}

/// The deploy intent set post endpoint.
///
/// Takes a signed vector of intents as a json payload.
async fn deploy_intent_set<S>(
    State(essential): State<Essential<S>>,
    Json(payload): Json<Signed<Vec<Intent>>>,
) -> Result<Json<ContentAddress>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let address = essential.deploy_intent_set(payload).await?;
    Ok(Json(address))
}

/// The submit solution post endpoint.
///
/// Takes a signed solution as a json payload.
async fn submit_solution<S>(
    State(essential): State<Essential<S>>,
    Json(payload): Json<Signed<Solution>>,
) -> Result<Json<Hash>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let hash = essential.submit_solution(payload).await?;
    Ok(Json(hash))
}

/// The get intent set get endpoint.
///
/// Takes a content address (encoded as base64) as a path parameter.
async fn get_intent_set<S>(
    State(essential): State<Essential<S>>,
    Path(address): Path<String>,
) -> Result<Json<Option<Signed<Vec<Intent>>>>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let address = ContentAddress(
        URL_SAFE
            .decode(address)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("Content Address wrong size"))?,
    );
    let set = essential.get_intent_set(&address).await?;
    Ok(Json(set))
}

/// The get intent get endpoint.
///
/// Takes a set content address and an intent content address as path parameters.
/// Both are encoded as base64.
async fn get_intent<S>(
    State(essential): State<Essential<S>>,
    Path((set, address)): Path<(String, String)>,
) -> Result<Json<Option<Intent>>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
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

/// The list intent sets get endpoint.
///
/// Takes optional time range and page as query parameters.
async fn list_intent_sets<S>(
    State(essential): State<Essential<S>>,
    time_range: Option<Query<TimeRange>>,
    page: Option<Query<Page>>,
) -> Result<Json<Vec<Vec<Intent>>>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let time_range =
        time_range.map(|range| Duration::from_secs(range.start)..Duration::from_secs(range.end));

    let sets = essential
        .list_intent_sets(time_range, page.map(|p| p.page as usize))
        .await?;
    Ok(Json(sets))
}

/// The list winning blocks get endpoint.
///
/// Takes optional time range and page as query parameters.
async fn list_winning_blocks<S>(
    State(essential): State<Essential<S>>,
    time_range: Option<Query<TimeRange>>,
    page: Option<Query<Page>>,
) -> Result<Json<Vec<Block>>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let time_range =
        time_range.map(|range| Duration::from_secs(range.start)..Duration::from_secs(range.end));

    let blocks = essential
        .list_winning_blocks(time_range, page.map(|p| p.page as usize))
        .await?;
    Ok(Json(blocks))
}

/// The list solutions pool get endpoint.
async fn list_solutions_pool<S>(
    State(essential): State<Essential<S>>,
) -> Result<Json<Vec<Signed<Solution>>>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let solutions = essential.list_solutions_pool().await?;
    Ok(Json(solutions))
}

/// The query state get endpoint.
///
/// Takes a content address and a key as path parameters.
/// Both are encoded as base64.
///
/// Note the key is a 32 byte array of u8 encoded as base64.
async fn query_state<S>(
    State(essential): State<Essential<S>>,
    Path((address, key)): Path<(String, String)>,
) -> Result<Json<Option<Word>>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
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

    // Convert the key to four words.
    let key = word_4_from_u8_32(key);

    let state = essential.query_state(&address, &key).await?;
    Ok(Json(state))
}

/// The solution outcome get endpoint.
///
/// Takes a solution hash as a path parameter.
///
/// The solution hash is a 32 byte array encoded as base64.
async fn solution_outcome<S>(
    State(essential): State<Essential<S>>,
    Path(hash): Path<String>,
) -> Result<Json<Option<SolutionOutcome>>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let hash: Hash = URL_SAFE
        .decode(hash)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("Hash is wrong size"))?;
    let outcome = essential.solution_outcome(&hash).await?;
    Ok(Json(outcome))
}

/// The check solution post endpoint.
///
/// Takes a signed solution as a json payload.
async fn check_solution<S>(
    State(essential): State<Essential<S>>,
    Json(payload): Json<Signed<Solution>>,
) -> Result<Json<CheckSolutionOutput>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let outcome = essential.check_solution(payload).await?;
    Ok(Json(outcome))
}

/// The check solution with data post endpoint.
///
/// Takes a signed solution and a list of intents as a json payload.
async fn check_solution_with_data<S>(
    State(essential): State<Essential<S>>,
    Json(payload): Json<CheckSolution>,
) -> Result<Json<CheckSolutionOutput>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let outcome = essential
        .check_solution_with_data(payload.solution, payload.intents)
        .await?;
    Ok(Json(outcome))
}

/// Shutdown the server manually or on ctrl-c.
async fn shutdown(rx: Option<oneshot::Receiver<()>>) {
    // The manual signal is used to shutdown the server.
    let manual = async {
        match rx {
            Some(rx) => {
                rx.await.ok();
            }
            None => futures::future::pending().await,
        }
    };

    // The ctrl-c signal is used to shutdown the server.
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl-c");
    };

    // Wait for either signal.
    tokio::select! {
        _ = manual => {},
        _ = ctrl_c => {},
    }
}

struct Error(anyhow::Error);

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        // Return an internal server error with the error message.
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
