// TODO: remove this
#![allow(unused_variables)]
#![deny(missing_docs)]
//! # Rqlite storage
//! This uses a remote rqlite server to store data.

use anyhow::{bail, ensure};
use essential_hash::hash;
use essential_state_read_vm::StateRead;
use essential_storage::{
    failed_solution::{FailedSolution, SolutionFailReason, SolutionOutcomes},
    key_range, CommitData, QueryState, StateStorage, Storage,
};
use essential_types::{
    contract::{Contract, SignedContract},
    predicate, Block, ContentAddress, Hash, Key, Word,
};
use futures::FutureExt;
use std::{pin::Pin, sync::Arc, time::Duration};
use thiserror::Error;

use values::{single_value, QueryValues};

const CREATE_TABLES_RETRY_DELAY: Duration = Duration::from_secs(1);

#[cfg(test)]
mod test_encode_decode;
mod values;

/// Amount of values returned in a single page.
const PAGE_SIZE: usize = 100;

/// The key to results in the query values.
const RESULTS_KEY: &str = "results";

/// The key to errors in the results of a query.
const ERROR_KEY: &str = "errors";

const MAX_DB_CONNECTIONS: usize = 400;

#[derive(Clone)]
/// Rqlite storage
/// Safe to clone connection to the rqlite server.
pub struct RqliteStorage {
    http: Db,
    server: reqwest::Url,
}

#[derive(Clone)]
struct Db {
    semaphore: Arc<tokio::sync::Semaphore>,
    http: reqwest::Client,
}

/// Encodes a type into blob data which is then hex encoded.
fn encode<T: serde::Serialize>(value: &T) -> String {
    let value = postcard::to_allocvec(value).expect("How can this fail?");
    hex::encode_upper(value)
}

/// Decodes a hex encoded blob into a type.
fn decode<T: serde::de::DeserializeOwned>(value: &str) -> anyhow::Result<T> {
    let value = hex::decode(value)?;
    Ok(postcard::from_bytes(&value)?)
}

/// Constructs an SQL statement ready for execution in the form of a list of JSON values,
/// where the first element is the SQL string at the specified path and the following
/// elements are its arguments.
///
/// Owned variant creates a Vec instead of a slice.
///
/// Named variant creates a map of arguments.
macro_rules! include_sql {
    ($name:expr $(, $($arg:expr),*)?) => {
        &[
            serde_json::Value::String(include_str!(concat!(concat!(env!("CARGO_MANIFEST_DIR"), "/sql/", $name))).to_string()),
            $( $($arg.into()),* )?
        ][..]
    };
    (owned $name:expr $(, $($arg:expr),*)?) => {
        vec![
            serde_json::Value::String(include_str!(concat!(concat!(env!("CARGO_MANIFEST_DIR"), "/sql/", $name))).to_string()),
            $( $($arg.into()),* )?
        ]
    };
    (named $sql:expr $(, $($name:expr => $arg:expr),*)?) => {
        &[
            serde_json::Value::String(include_str!(concat!(concat!(env!("CARGO_MANIFEST_DIR"), "/sql/", $sql))).to_string()),
            [$( $(($name.into(), $arg.into())),* )?].into_iter().collect::<serde_json::Map<String, serde_json::Value>>().into()
        ][..]
    };
}

impl Db {
    async fn acquire<F, Fut, R>(&self, f: F) -> anyhow::Result<R>
    where
        F: FnOnce(reqwest::Client) -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<R>>,
    {
        let permit = self.semaphore.acquire().await?;
        f(self.http.clone()).await
    }
}

impl RqliteStorage {
    /// Create a new rqlite storage from the rqlite server address.
    pub async fn new(server: &str) -> anyhow::Result<Self> {
        let s = Self {
            http: Db {
                semaphore: Arc::new(tokio::sync::Semaphore::new(MAX_DB_CONNECTIONS)),
                http: reqwest::Client::new(),
            },
            server: reqwest::Url::parse(server)?,
        };
        while let Err(err) = s.create_tables().await {
            #[cfg(feature = "tracing")]
            tracing::warn!("Failed to create tables: {:?}. Retrying...", err);
            tokio::time::sleep(CREATE_TABLES_RETRY_DELAY).await;
        }
        Ok(s)
    }

    /// Create all the tables in the rqlite server.
    /// This is idempotent.
    pub async fn create_tables(&self) -> anyhow::Result<()> {
        let creates = &[
            include_sql!("create/predicates.sql"),
            include_sql!("create/contracts.sql"),
            include_sql!("create/contract_pairing.sql"),
            include_sql!("create/solutions.sql"),
            include_sql!("create/solutions_pool.sql"),
            include_sql!("create/solved.sql"),
            include_sql!("create/contract_state.sql"),
            include_sql!("create/batch.sql"),
            include_sql!("create/failed_solutions.sql"),
            include_sql!("index/solved_batch_id.sql"),
            include_sql!("index/solved_content_hash.sql"),
            include_sql!("index/failed_solutions_content_hash.sql"),
        ];
        self.execute(&creates[..]).await
    }

    /// Execute a sql statement on the rqlite server.
    async fn execute(&self, sql: &[&[serde_json::Value]]) -> anyhow::Result<()> {
        let url = self.server.join("/db/execute?transaction")?;
        let r = self
            .http
            .acquire(|http| async move { Ok(http.post(url).json(&sql).send().await?) })
            .await?;
        ensure!(
            r.status().is_success(),
            "failed to execute {:?}",
            r.text().await?
        );
        let result: serde_json::Map<String, serde_json::Value> = r.json().await?;

        handle_errors(&result, sql)?;
        Ok(())
    }

    async fn execute_query(&self, sql: &[&[serde_json::Value]]) -> anyhow::Result<QueryValues> {
        let url = self.server.join("/db/request?transaction&level=strong")?;
        let r = self
            .http
            .acquire(|http| async move { Ok(http.post(url).json(&sql).send().await?) })
            .await?;
        ensure!(
            r.status().is_success(),
            "failed to query values {:?}",
            r.text().await?
        );

        let value: serde_json::Map<String, serde_json::Value> = r.json().await?;
        handle_errors(&value, sql)?;
        values::map_query_to_query_values(value)
    }

    /// Execute a sql statement on the rqlite server and return a list of words.
    /// This is useful for mixing word queries in the same transaction
    /// as an execute statement.
    async fn execute_query_words(&self, sql: &[&[serde_json::Value]]) -> anyhow::Result<Vec<Word>> {
        let url = self.server.join("/db/request?transaction&level=strong")?;
        let r = self
            .http
            .acquire(|http| async move { Ok(http.post(url).json(&sql).send().await?) })
            .await?;
        ensure!(
            r.status().is_success(),
            "failed to execute query {:?}",
            r.text().await?
        );

        let value: serde_json::Map<String, serde_json::Value> = r.json().await?;
        handle_errors(&value, sql)?;
        values::assert_row_changed(&value, sql)?;
        values::map_execute_to_values(value)
    }

    /// Query a sql statement on the rqlite server.
    /// Returns `QueryValues` which is a collection of rows and columns.
    async fn query_values(&self, sql: &[&[serde_json::Value]]) -> anyhow::Result<QueryValues> {
        let url = self.server.join("/db/query?transaction")?;
        let r = self
            .http
            .acquire(|http| async move { Ok(http.post(url).json(&sql).send().await?) })
            .await?;
        ensure!(
            r.status().is_success(),
            "failed to query values {:?}",
            r.text().await?
        );

        let value: serde_json::Map<String, serde_json::Value> = r.json().await?;
        handle_errors(&value, sql)?;
        values::map_query_to_query_values(value)
    }
}

fn handle_errors(
    result: &serde_json::Map<String, serde_json::Value>,
    sql: &[&[serde_json::Value]],
) -> anyhow::Result<()> {
    if let Some(serde_json::Value::Array(results)) = result.get(RESULTS_KEY) {
        for result in results {
            if let Some(serde_json::Value::String(error)) = result.get(ERROR_KEY) {
                anyhow::bail!("failed to execute {:?} {:?}", sql, error);
            }
        }
    }
    Ok(())
}

impl StateStorage for RqliteStorage {
    async fn update_state(
        &self,
        address: &essential_types::ContentAddress,
        key: &essential_types::Key,
        value: Vec<essential_types::Word>,
    ) -> anyhow::Result<Vec<essential_types::Word>> {
        let address = encode(address);
        let key = encode(key);
        let value = encode(&value);
        if value.is_empty() {
            // Delete the value and return the existing value if it exists.
            let inserts = &[
                include_sql!("query/get_state.sql", address.clone(), key.clone()),
                include_sql!("update/delete_state.sql", address, key),
            ];
            self.execute_query_words(&inserts[..]).await
        } else {
            // Update the value and return the existing value if it exists.
            let inserts = &[
                include_sql!("query/get_state.sql", address.clone(), key.clone()),
                include_sql!("update/update_state.sql", key, value, address),
            ];
            self.execute_query_words(&inserts[..]).await
        }
    }

    async fn update_state_batch<U>(&self, updates: U) -> anyhow::Result<Vec<Vec<Word>>>
    where
        U: IntoIterator<Item = (ContentAddress, essential_types::Key, Vec<Word>)> + Send,
    {
        let sql: Vec<_> = updates
            .into_iter()
            .flat_map(|(address, key, value)| {
                let address = encode(&address);
                let key = encode(&key);
                let value = encode(&value);
                if value.is_empty() {
                    // Delete the value and return the existing value if it exists.
                    [
                        include_sql!(owned "query/get_state.sql", address.clone(), key.clone()),
                        include_sql!(owned "update/delete_state.sql", address, key),
                    ]
                } else {
                    // Update the value and return the existing value if it exists.
                    [
                        include_sql!(owned "query/get_state.sql", address.clone(), key.clone()),
                        include_sql!(owned "update/update_state.sql", key, value, address),
                    ]
                }
            })
            .collect();

        // Return early if there are no updates.
        if sql.is_empty() {
            return Ok(Vec::new());
        }

        // TODO: Is there a way to avoid this?
        // Maybe create an owned version of execute.
        let sql: Vec<&[serde_json::Value]> = sql.iter().map(|v| &v[..]).collect();
        let queries = self.execute_query(&sql).await?;
        values::map_execute_to_multiple_values(queries)
    }
}

impl QueryState for RqliteStorage {
    async fn query_state(
        &self,
        address: &essential_types::ContentAddress,
        key: &essential_types::Key,
    ) -> anyhow::Result<Vec<essential_types::Word>> {
        let address = encode(address);
        let key = encode(key);
        let sql = &[include_sql!("query/get_state.sql", address, key)];
        let queries = self.query_values(sql).await?;
        match single_value(&queries) {
            Some(serde_json::Value::String(v)) => decode(v),
            None => Ok(Vec::new()),
            _ => bail!("State stored incorrectly"),
        }
    }
}

impl Storage for RqliteStorage {
    async fn insert_contract(&self, mut contract: SignedContract) -> anyhow::Result<()> {
        // Get the time this contract was created at.
        let created_at = std::time::SystemTime::now();
        let unix_time = created_at.duration_since(std::time::UNIX_EPOCH)?;

        contract.contract.sort_by_key(essential_hash::content_addr);

        // Encode the data into hex blobs.
        let contract_addr = essential_hash::contract_addr::from_contract(&contract.contract);
        let address = encode(&contract_addr);
        let signature = encode(&contract.signature);

        // For each predicate, insert the predicate and the contract pairing.
        let contract = contract.contract.iter().flat_map(|predicate| {
            let hash = encode(&essential_hash::content_addr(&predicate));
            let predicate = encode(&predicate);
            [
                include_sql!(
                    owned
                    "insert/contract.sql",
                    predicate,
                    hash.clone()
                ),
                include_sql!(
                    owned
                    "insert/contract_pairing.sql",
                    address.clone(),
                    hash
                ),
            ]
        });

        // Insert the contract and storage layout then the contract and pairings.
        let mut inserts = vec![include_sql!(owned
            "insert/contract.sql",
            address.clone(),
            signature,
            unix_time.as_secs(),
            unix_time.subsec_nanos()
        )];
        inserts.extend(contract);

        // TODO: Is there a way to avoid this?
        // Maybe create an owned version of execute.
        let inserts: Vec<&[serde_json::Value]> = inserts.iter().map(|v| v.as_slice()).collect();
        self.execute(&inserts[..]).await
    }

    async fn insert_solution_into_pool(
        &self,
        solution: essential_types::solution::Solution,
    ) -> anyhow::Result<()> {
        let hash = encode(&hash(&solution));
        let solution = encode(&solution);

        let inserts = &[
            include_sql!("insert/solutions.sql", hash.clone(), solution),
            include_sql!("insert/solutions_pool.sql", hash),
        ];
        self.execute(&inserts[..]).await
    }

    async fn move_solutions_to_solved(
        &self,
        solutions: &[essential_types::Hash],
    ) -> anyhow::Result<()> {
        if solutions.is_empty() {
            return Ok(());
        }

        let sql = move_solutions_to_solved(solutions)?;

        // TODO: Is there a way to avoid this?
        // Maybe create an owned version of execute.
        let sql: Vec<&[serde_json::Value]> = sql.iter().map(|v| v.as_slice()).collect();
        self.execute(&sql[..]).await
    }

    async fn move_solutions_to_failed(
        &self,
        solutions: &[(Hash, SolutionFailReason)],
    ) -> anyhow::Result<()> {
        if solutions.is_empty() {
            return Ok(());
        }

        let sql = move_solutions_to_failed(solutions)?;

        // TODO: Is there a way to avoid this?
        // Maybe create an owned version of execute.
        let sql: Vec<&[serde_json::Value]> = sql.iter().map(|v| v.as_slice()).collect();
        self.execute(&sql[..]).await
    }

    async fn get_predicate(
        &self,
        address: &essential_types::PredicateAddress,
    ) -> anyhow::Result<Option<essential_types::predicate::Predicate>> {
        let contract = encode(&address.contract);
        let predicate_hash = encode(&address.predicate);
        let sql = &[include_sql!(
            "query/get_predicate.sql",
            contract,
            predicate_hash
        )];
        let queries = self.query_values(sql).await?;

        // Expecting single query, single row, single column
        match single_value(&queries) {
            Some(serde_json::Value::String(predicate)) => Ok(Some(decode(predicate)?)),
            None => Ok(None),
            _ => bail!("Predicate stored incorrectly"),
        }
    }

    async fn get_contract(
        &self,
        address: &essential_types::ContentAddress,
    ) -> anyhow::Result<Option<SignedContract>> {
        let address = encode(address);
        let sql = &[
            include_sql!("query/get_contract_signature.sql", address.clone()),
            include_sql!("query/get_contract.sql", address),
        ];
        let queries = self.query_values(sql).await?;
        values::get_contract(queries)
    }

    async fn list_contracts(
        &self,
        time_range: Option<std::ops::Range<std::time::Duration>>,
        page: Option<usize>,
    ) -> anyhow::Result<Vec<Contract>> {
        let page = page.unwrap_or(0);
        let queries = match time_range {
            Some(range) => {
                let sql = &[include_sql!(
                    "query/list_contracts_by_time.sql",
                    range.start.as_secs(),
                    range.start.subsec_nanos(),
                    range.end.as_secs(),
                    range.end.subsec_nanos(),
                    PAGE_SIZE,
                    page
                )];
                self.query_values(sql).await?
            }
            None => {
                let sql = &[
                    include_sql!(named "query/list_contracts.sql", "page_size" => PAGE_SIZE, "page_number" => page),
                ];
                self.query_values(sql).await?
            }
        };
        values::list_contracts(queries)
    }

    async fn list_solutions_pool(
        &self,
        page: Option<usize>,
    ) -> anyhow::Result<Vec<essential_types::solution::Solution>> {
        let page = page.unwrap_or(0);
        let sql = &[
            include_sql!(named "query/list_solutions_pool.sql", "page_size" => PAGE_SIZE, "page_number" => page),
        ];
        let queries = self.query_values(sql).await?;
        values::list_solutions_pool(queries)
    }

    async fn list_failed_solutions_pool(
        &self,
        page: Option<usize>,
    ) -> anyhow::Result<Vec<FailedSolution>> {
        let page = page.unwrap_or(0);
        let sql = &[
            include_sql!(named "query/list_failed_solutions.sql", "page_size" => PAGE_SIZE, "page_number" => page),
        ];
        let queries = self.query_values(sql).await?;
        values::list_failed_solutions(queries)
    }

    async fn list_winning_blocks(
        &self,
        time_range: Option<std::ops::Range<std::time::Duration>>,
        page: Option<usize>,
    ) -> anyhow::Result<Vec<Block>> {
        let page = page.unwrap_or(0);
        let queries = match time_range {
            Some(range) => {
                let sql = &[include_sql!(named "query/list_winning_batches_by_time.sql",
                    "page_size" => PAGE_SIZE,
                    "page_number" => page,
                    "start_seconds" => range.start.as_secs(),
                    "start_nanos" => range.start.subsec_nanos(),
                    "end_seconds" => range.end.as_secs(),
                    "end_nanos" => range.end.subsec_nanos()
                )];
                self.query_values(sql).await?
            }
            None => {
                let sql = &[
                    include_sql!(named "query/list_winning_batches.sql", "page_size" => PAGE_SIZE, "page_number" => page),
                ];
                self.query_values(sql).await?
            }
        };
        values::list_winning_blocks(queries)
    }

    async fn get_solution(&self, solution_hash: Hash) -> anyhow::Result<Option<SolutionOutcomes>> {
        let hash = encode(&solution_hash);
        let sql = &[
            include_sql!("query/get_solution.sql", hash.clone()),
            include_sql!("query/get_solution_outcomes.sql", hash.clone(), hash),
        ];
        let queries = self.query_values(sql).await?;
        values::get_solution(queries)
    }

    async fn prune_failed_solutions(&self, older_than: Duration) -> anyhow::Result<()> {
        let sql = &[include_sql!(
            "update/prune_failed.sql",
            older_than.as_secs()
        )];
        self.execute(&sql[..]).await
    }

    fn commit_block(
        &self,
        data: CommitData,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        let CommitData {
            failed,
            solved,
            state_updates,
        } = data;
        let r = if !failed.is_empty() {
            move_solutions_to_failed(failed)
        } else {
            Ok(Vec::new())
        };

        let r = r.and_then(|mut sql| {
            let r = if !solved.is_empty() {
                move_solutions_to_solved(solved)
            } else {
                Ok(Vec::new())
            };
            match r {
                Ok(s) => {
                    sql.extend(s);
                    sql.extend(update_state_batch(state_updates));
                    Ok(sql)
                }
                Err(e) => Err(e),
            }
        });

        async move {
            let sql = r?;
            if sql.is_empty() {
                return Ok(());
            }
            // TODO: Is there a way to avoid this?
            // Maybe create an owned version of execute.
            let sql: Vec<&[serde_json::Value]> = sql.iter().map(|v| v.as_slice()).collect();
            self.execute(&sql[..]).await
        }
    }
}

fn move_solutions_to_failed(
    solutions: &[(Hash, SolutionFailReason)],
) -> anyhow::Result<Vec<Vec<serde_json::Value>>> {
    let created_at = std::time::SystemTime::now();
    let unix_time = created_at.duration_since(std::time::UNIX_EPOCH)?;
    Ok(solutions
        .iter()
        .flat_map(|(hash, reason)| {
            let hash = encode(hash);
            let reason = encode(reason);
            [
                include_sql!(owned "insert/copy_to_failed.sql",
                    reason,
                    unix_time.as_secs(),
                    unix_time.subsec_nanos(),
                    hash.clone()
                ),
                include_sql!(owned "update/delete_from_solutions_pool.sql", hash),
            ]
        })
        .collect())
}

fn move_solutions_to_solved(solutions: &[Hash]) -> anyhow::Result<Vec<Vec<serde_json::Value>>> {
    if solutions.is_empty() {
        return Ok(Vec::new());
    }
    let created_at = std::time::SystemTime::now();
    let unix_time = created_at.duration_since(std::time::UNIX_EPOCH)?;
    let inserts = solutions.iter().flat_map(|hash| {
        let hash = encode(hash);
        [
            include_sql!(owned "insert/copy_to_solved.sql", hash.clone()),
            include_sql!(owned "update/delete_from_solutions_pool.sql", hash),
        ]
    });
    let mut sql = vec![include_sql!(
        owned "insert/batch.sql",
        unix_time.as_secs(),
        unix_time.subsec_nanos()
    )];
    sql.extend(inserts);
    sql.push(include_sql!(owned "update/delete_empty_batch.sql"));
    Ok(sql)
}

fn update_state_batch<U>(updates: U) -> Vec<Vec<serde_json::Value>>
where
    U: IntoIterator<Item = (ContentAddress, essential_types::Key, Vec<Word>)>,
{
    updates
        .into_iter()
        .flat_map(|(address, key, value)| {
            let address = encode(&address);
            let key = encode(&key);
            let value = encode(&value);
            if value.is_empty() {
                // Delete the value and return the existing value if it exists.
                [
                    include_sql!(owned "query/get_state.sql", address.clone(), key.clone()),
                    include_sql!(owned "update/delete_state.sql", address, key),
                ]
            } else {
                // Update the value and return the existing value if it exists.
                [
                    include_sql!(owned "query/get_state.sql", address.clone(), key.clone()),
                    include_sql!(owned "update/update_state.sql", key, value, address),
                ]
            }
        })
        .collect()
}

/// Error for rqlite read.
#[derive(Debug, Error)]
pub enum RqliteError {
    /// Error during read
    #[error("failed to read")]
    ReadError(#[from] anyhow::Error),
}

impl StateRead for RqliteStorage {
    type Error = RqliteError;

    type Future =
        Pin<Box<dyn std::future::Future<Output = Result<Vec<Vec<Word>>, Self::Error>> + Send>>;

    fn key_range(&self, contract_addr: ContentAddress, key: Key, num_words: usize) -> Self::Future {
        let storage = self.clone();
        async move { key_range(&storage, contract_addr, key, num_words).await }.boxed()
    }
}
