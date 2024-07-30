#![deny(missing_docs)]
//! # Server
//!
//! A simple REST server for the Essential platform.

use anyhow::anyhow;
use axum::{
    extract::{Path, Query, State},
    response::{
        sse::{Event, KeepAlive},
        IntoResponse, Sse,
    },
    routing::{get, post},
    Json, Router,
};
use essential_server::{CheckSolutionOutput, Essential, SolutionOutcome, StateRead, Storage};
use essential_server_types::{CheckSolution, QueryStateReads, QueryStateReadsOutput};
use essential_types::{
    contract::{Contract, SignedContract},
    convert::word_from_bytes,
    predicate::Predicate,
    solution::Solution,
    Block, ContentAddress, PredicateAddress, Word,
};
use futures::{Stream, StreamExt};
use hyper::body::Incoming;
use hyper_util::rt::{TokioExecutor, TokioIo};
use serde::Deserialize;
use std::{net::SocketAddr, time::Duration};
use tokio::{
    net::{TcpListener, ToSocketAddrs},
    sync::oneshot,
    task::JoinSet,
};
use tower::Service;
use tower_http::cors::CorsLayer;

const MAX_CONNECTIONS: usize = 2000;

#[derive(Debug, Clone)]
/// Server configuration.
pub struct Config {
    /// Whether the rest server should build blocks
    /// or just serve requests.
    /// Default is `true`.
    pub build_blocks: bool,
    /// Essential server configuration.
    pub server_config: essential_server::Config,
}

#[derive(Deserialize)]
/// Type to deserialize a time range query parameters.
struct TimeRange {
    /// Start of the time range in seconds.
    start: u64,
    /// End of the time range in seconds.
    end: u64,
}

#[derive(Deserialize)]
/// Type to deserialize a time query parameters.
struct Time {
    /// Time in seconds.
    time: u64,
}

#[derive(Deserialize)]
/// Type to deserialize a page query parameter.
struct Page {
    /// The page number to start from.
    page: u64,
}

#[derive(Deserialize)]
/// Type to deserialize a block number query parameter.
struct BlockNumber {
    /// The block number to start from.
    block: u64,
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
    config: Config,
) -> anyhow::Result<()>
where
    A: ToSocketAddrs,
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    // Spawn essential and get the handle.
    let handle = if config.build_blocks {
        Some(essential.clone().spawn(config.server_config)?)
    } else {
        None
    };

    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([http::Method::GET, http::Method::POST, http::Method::OPTIONS])
        .allow_headers([http::header::CONTENT_TYPE]);

    // Create all the endpoints.
    let app = Router::new()
        .route("/", get(health_check))
        .route("/deploy-contract", post(deploy_contract))
        .route("/get-contract/:address", get(get_contract))
        .route("/get-predicate/:contract/:address", get(get_predicate))
        .route("/list-contracts", get(list_contracts))
        .route("/subscribe-contracts", get(subscribe_contracts))
        .route("/submit-solution", post(submit_solution))
        .route("/list-solutions-pool", get(list_solutions_pool))
        .route("/query-state/:address/:key", get(query_state))
        .route("/list-blocks", get(list_blocks))
        .route("/subscribe-blocks", get(subscribe_blocks))
        .route("/solution-outcome/:hash", get(solution_outcome))
        .route("/check-solution", post(check_solution))
        .route(
            "/check-solution-with-contracts",
            post(check_solution_with_contracts),
        )
        .route("/query-state-reads", post(query_state_reads))
        .layer(cors)
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
    serve(app, listener, shutdown_rx).await;

    // After the server is done, shutdown essential.
    if let Some(handle) = handle {
        handle.shutdown().await?;
    }

    Ok(())
}

async fn serve(app: Router, listener: TcpListener, shutdown_rx: Option<oneshot::Receiver<()>>) {
    let shut = shutdown(shutdown_rx);
    tokio::pin!(shut);

    let mut conn_contract = JoinSet::new();
    // Continuously accept new connections up to max connections.
    loop {
        // Accept a new connection or wait for a shutdown signal.
        let (socket, remote_addr) = tokio::select! {
            _ = &mut shut => {
                break;
            }
            v = listener.accept() => {
                match v {
                    Ok(v) => v,
                    Err(err) => {
                        #[cfg(feature = "tracing")]
                        tracing::trace!("Failed to accept connection {}", err);
                        continue;
                    }
                }
            }
        };

        #[cfg(feature = "tracing")]
        tracing::trace!("Accepted new connection from: {}", remote_addr);

        // We don't need to call `poll_ready` because `Router` is always ready.
        let tower_service = app.clone();

        // Spawn a task to handle the connection. That way we can handle multiple connections
        // concurrently.

        conn_contract.spawn(async move {
            // Hyper has its own `AsyncRead` and `AsyncWrite` traits and doesn't use tokio.
            // `TokioIo` converts between them.
            let socket = TokioIo::new(socket);

            // Hyper also has its own `Service` trait and doesn't use tower. We can use
            // `hyper::service::service_fn` to create a hyper `Service` that calls our app through
            // `tower::Service::call`.
            let hyper_service =
                hyper::service::service_fn(move |request: axum::extract::Request<Incoming>| {
                    // We have to clone `tower_service` because hyper's `Service` uses `&self` whereas
                    // tower's `Service` requires `&mut self`.
                    //
                    // We don't need to call `poll_ready` since `Router` is always ready.
                    tower_service.clone().call(request)
                });

            // `TokioExecutor` tells hyper to use `tokio::spawn` to spawn tasks.
            let conn =
                hyper_util::server::conn::auto::Builder::new(TokioExecutor::new()).http2_only();
            let conn = conn.serve_connection(socket, hyper_service);
            let _ = conn.await;
        });

        // Wait for existing connection to close or wait for a shutdown signal.
        if conn_contract.len() > MAX_CONNECTIONS {
            #[cfg(feature = "tracing")]
            tracing::info!("Max number of connections reached: {}", MAX_CONNECTIONS);
            tokio::select! {
                _ = &mut shut => {
                    break;
                }
                _ = conn_contract.join_next() => {},

            }
        }
    }
}

/// The return a health check response.
async fn health_check() {}

/// The deploy contract post endpoint.
///
/// Takes a signed vector of contract as a json payload.
async fn deploy_contract<S>(
    State(essential): State<Essential<S>>,
    Json(payload): Json<SignedContract>,
) -> Result<Json<ContentAddress>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let address = essential.deploy_contract(payload).await?;
    Ok(Json(address))
}

/// The submit solution post endpoint.
///
/// Takes a signed solution as a json payload.
async fn submit_solution<S>(
    State(essential): State<Essential<S>>,
    Json(payload): Json<Solution>,
) -> Result<Json<ContentAddress>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let hash = essential.submit_solution(payload).await?;
    Ok(Json(hash))
}

/// The get contract get endpoint.
///
/// Takes a content address (encoded as hex) as a path parameter.
async fn get_contract<S>(
    State(essential): State<Essential<S>>,
    Path(address): Path<String>,
) -> Result<Json<Option<SignedContract>>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let address: ContentAddress = address
        .parse()
        .map_err(|e| anyhow!("failed to parse contract content address: {e}"))?;
    let contract = essential.get_contract(&address).await?;
    Ok(Json(contract))
}

/// The get predicate get endpoint.
///
/// Takes a contract content address and a predicate content address as path parameters.
/// Both are encoded as hex.
async fn get_predicate<S>(
    State(essential): State<Essential<S>>,
    Path((contract, address)): Path<(String, String)>,
) -> Result<Json<Option<Predicate>>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let contract: ContentAddress = contract
        .parse()
        .map_err(|e| anyhow!("failed to parse contract content address: {e}"))?;
    let predicate: ContentAddress = address
        .parse()
        .map_err(|e| anyhow!("failed to parse predicate content address: {e}"))?;
    let predicate = essential
        .get_predicate(&PredicateAddress {
            contract,
            predicate,
        })
        .await?;
    Ok(Json(predicate))
}

/// The list contracts get endpoint.
///
/// Takes optional time range and page as query parameters.
async fn list_contracts<S>(
    State(essential): State<Essential<S>>,
    time_range: Option<Query<TimeRange>>,
    page: Option<Query<Page>>,
) -> Result<Json<Vec<Contract>>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let time_range =
        time_range.map(|range| Duration::from_secs(range.start)..Duration::from_secs(range.end));

    let contracts = essential
        .list_contracts(time_range, page.map(|p| p.page as usize))
        .await?;
    Ok(Json(contracts))
}

/// The subscribe contracts get endpoint.
///
/// Takes optional time and page as query parameters.
async fn subscribe_contracts<S>(
    State(essential): State<Essential<S>>,
    time: Option<Query<Time>>,
    page: Option<Query<Page>>,
) -> Sse<impl Stream<Item = Result<Event, StdError>>>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let time = time.map(|t| Duration::from_secs(t.time));

    let contracts = essential.subscribe_contracts(time, page.map(|p| p.page as usize));
    Sse::new(
        contracts
            .map::<Result<_, Error>, _>(|contract| Ok(Event::default().json_data(contract?)?))
            .map(|r| r.map_err(StdError)),
    )
    .keep_alive(KeepAlive::default())
}

/// The list blocks get endpoint.
///
/// Takes optional time range and page as query parameters.
async fn list_blocks<S>(
    State(essential): State<Essential<S>>,
    time_range: Option<Query<TimeRange>>,
    block: Option<Query<BlockNumber>>,
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
        .list_blocks(
            time_range,
            block.map(|b| b.block),
            page.map(|p| p.page as usize),
        )
        .await?;
    Ok(Json(blocks))
}

/// The subscribe blocks get endpoint.
///
/// Takes optional time and page as query parameters.
async fn subscribe_blocks<S>(
    State(essential): State<Essential<S>>,
    time: Option<Query<Time>>,
    block: Option<Query<BlockNumber>>,
    page: Option<Query<Page>>,
) -> Sse<impl Stream<Item = Result<Event, StdError>>>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let time = time.map(|time| Duration::from_secs(time.time));

    let blocks =
        essential.subscribe_blocks(time, block.map(|b| b.block), page.map(|p| p.page as usize));
    Sse::new(
        blocks
            .map::<Result<_, Error>, _>(|block| Ok(Event::default().json_data(block?)?))
            .map(|r| r.map_err(StdError)),
    )
    .keep_alive(KeepAlive::default())
}

/// The list solutions pool get endpoint.
async fn list_solutions_pool<S>(
    State(essential): State<Essential<S>>,
    page: Option<Query<Page>>,
) -> Result<Json<Vec<Solution>>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let solutions = essential
        .list_solutions_pool(page.map(|p| p.page as usize))
        .await?;
    Ok(Json(solutions))
}

/// The query state get endpoint.
///
/// Takes a content address and a byte array key as path parameters.
/// Both are encoded as hex.
async fn query_state<S>(
    State(essential): State<Essential<S>>,
    Path((address, key)): Path<(String, String)>,
) -> Result<Json<Vec<Word>>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let address: ContentAddress = address
        .parse()
        .map_err(|e| anyhow!("failed to parse contract content address: {e}"))?;
    let key: Vec<u8> = hex::decode(key).map_err(|e| anyhow!("failed to decode key: {e}"))?;

    // Convert the key to words.
    let key = key
        .chunks_exact(8)
        .map(|chunk| word_from_bytes(chunk.try_into().expect("Safe due to chunk size")))
        .collect::<Vec<_>>();

    let state = essential.query_state(&address, &key).await?;
    Ok(Json(state))
}

/// The solution outcome get endpoint.
///
/// Takes a solution content address as a path parameter encoded hex.
async fn solution_outcome<S>(
    State(essential): State<Essential<S>>,
    Path(address): Path<String>,
) -> Result<Json<Vec<SolutionOutcome>>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let address: ContentAddress = address
        .parse()
        .map_err(|e| anyhow!("failed to parse solution content address: {e}"))?;
    let outcome = essential.solution_outcome(&address.0).await?;
    Ok(Json(outcome))
}

/// The check solution post endpoint.
///
/// Takes a signed solution as a json payload.
async fn check_solution<S>(
    State(essential): State<Essential<S>>,
    Json(payload): Json<Solution>,
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
/// Takes a signed solution and a list of contract as a json payload.
async fn check_solution_with_contracts<S>(
    State(essential): State<Essential<S>>,
    Json(payload): Json<CheckSolution>,
) -> Result<Json<CheckSolutionOutput>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let outcome = essential
        .check_solution_with_contracts(payload.solution, payload.contracts)
        .await?;
    Ok(Json(outcome))
}

/// The query state reads post endpoint.
///
/// Takes a json state read query and returns the outcome
async fn query_state_reads<S>(
    State(essential): State<Essential<S>>,
    Json(payload): Json<QueryStateReads>,
) -> Result<Json<QueryStateReadsOutput>, Error>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let out = essential.query_state_reads(payload).await?;
    Ok(Json(out))
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

#[derive(Debug)]
struct Error(anyhow::Error);

#[derive(Debug)]
struct StdError(Error);

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

impl From<Error> for StdError {
    fn from(err: Error) -> Self {
        Self(err)
    }
}

impl std::error::Error for StdError {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Display for StdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            build_blocks: true,
            server_config: Default::default(),
        }
    }
}
