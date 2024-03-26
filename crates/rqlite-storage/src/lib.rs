#![deny(missing_docs)]
//! # Rqlite storage
//! This uses a remote rqlite server to store data.

use anyhow::ensure;
use base64::Engine;
use essential_types::{Block, ContentAddress, Signature, Signed, StorageLayout, Word};
use storage::Storage;
use utils::hash;

use values::{single_value, QueryValues};

mod values;

/// Amount of values returned in a single page.
const PAGE_SIZE: usize = 100;

/// The key to results in the query values.
const RESULTS_KEY: &str = "results";

/// The key to errors in the results of a query.
const ERROR_KEY: &str = "errors";

#[derive(Clone)]
/// Rqlite storage
/// Safe to clone connection to the rqlite server.
pub struct RqliteStorage {
    http: reqwest::Client,
    server: reqwest::Url,
}

/// Encodes a type into blob data which is then base64 encoded.
fn encode<T: serde::Serialize>(value: &T) -> String {
    let value = postcard::to_allocvec(value).expect("How can this fail?");
    base64::engine::general_purpose::STANDARD.encode(value)
}

/// Decodes a base64 encoded blob into a type.
fn decode<T: serde::de::DeserializeOwned>(value: &str) -> anyhow::Result<T> {
    let value = base64::engine::general_purpose::STANDARD.decode(value)?;
    Ok(postcard::from_bytes(&value)?)
}

/// Encodes a type into blob data which is then base64 encoded.
fn encode_signature(sig: &Signature) -> String {
    let value = postcard::to_allocvec(&sig).expect("How can this fail?");
    base64::engine::general_purpose::STANDARD.encode(value)
}

/// Decodes a base64 encoded blob into a type.
fn decode_signature(value: &str) -> anyhow::Result<Signature> {
    let value = base64::engine::general_purpose::STANDARD.decode(value)?;
    let sig = postcard::from_bytes::<Signature>(&value)?;
    Ok(sig)
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

impl RqliteStorage {
    /// Create a new rqlite storage from the rqlite server address.
    pub async fn new(server: &str) -> anyhow::Result<Self> {
        let s = Self {
            http: reqwest::Client::new(),
            server: reqwest::Url::parse(server)?,
        };
        s.create_tables().await?;
        Ok(s)
    }

    /// Create all the tables in the rqlite server.
    /// This is idempotent.
    pub async fn create_tables(&self) -> anyhow::Result<()> {
        let creates = &[
            include_sql!("create/intents.sql"),
            include_sql!("create/intent_sets.sql"),
            include_sql!("create/intent_set_pairing.sql"),
            include_sql!("create/storage_layout.sql"),
            include_sql!("create/solutions_pool.sql"),
            include_sql!("create/solved.sql"),
            include_sql!("create/intent_state.sql"),
            include_sql!("create/eoa.sql"),
            include_sql!("create/eoa_state.sql"),
            include_sql!("create/batch.sql"),
        ];
        self.execute(&creates[..]).await
    }

    /// Execute a sql statement on the rqlite server.
    async fn execute(&self, sql: &[&[serde_json::Value]]) -> anyhow::Result<()> {
        let r = self
            .http
            .post(self.server.join("/db/execute?transaction")?)
            .json(&sql)
            .send()
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

    /// Execute a sql statement on the rqlite server and return a single word.
    /// This is useful for mixing word queries in the same transaction
    /// as an execute statement.
    async fn execute_query_word(
        &self,
        sql: &[&[serde_json::Value]],
    ) -> anyhow::Result<Option<Word>> {
        let r = self
            .http
            .post(self.server.join("/db/request?transaction&level=strong")?)
            .json(&sql)
            .send()
            .await?;
        ensure!(
            r.status().is_success(),
            "failed to execute query {:?}",
            r.text().await?
        );

        let value: serde_json::Map<String, serde_json::Value> = r.json().await?;
        handle_errors(&value, sql)?;
        values::map_execute_to_word(value)
    }

    /// Query a sql statement on the rqlite server.
    /// Returns `QueryValues` which is a collection of rows and columns.
    async fn query_values(&self, sql: &[&[serde_json::Value]]) -> anyhow::Result<QueryValues> {
        let r = self
            .http
            .post(self.server.join("/db/query?transaction&level=strong")?)
            .json(&sql)
            .send()
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

impl Storage for RqliteStorage {
    async fn insert_intent_set(
        &self,
        storage_layout: StorageLayout,
        intents: Signed<Vec<essential_types::intent::Intent>>,
    ) -> anyhow::Result<()> {
        // Get the time this intent set was created at.
        let created_at = std::time::SystemTime::now();
        let unix_time = created_at.duration_since(std::time::UNIX_EPOCH)?;

        // Encode the data into base64 blobs.
        let address = encode(&ContentAddress(hash(&intents.data)));
        let signature = encode_signature(&intents.signature);
        let storage_layout = encode(&storage_layout);

        // For each intent, insert the intent and the intent set pairing.
        let intents = intents.data.iter().flat_map(|intent| {
            let hash = encode(&hash(&intent));
            let intent = encode(&intent);
            [
                include_sql!(
                    owned
                    "insert/intents.sql",
                    intent,
                    hash.clone()
                ),
                include_sql!(
                    owned
                    "insert/intent_set_pairing.sql",
                    address.clone(),
                    hash
                ),
            ]
        });

        // Insert the intent set and storage layout then the intents and pairings.
        let mut inserts = vec![
            include_sql!(owned
                "insert/intent_set.sql",
                address.clone(),
                signature,
                unix_time.as_secs(),
                unix_time.subsec_nanos()
            ),
            include_sql!(owned "insert/storage_layout.sql", storage_layout, address.clone()),
        ];
        inserts.extend(intents);

        // TODO: Is there a way to avoid this?
        // Maybe create an owned version of execute.
        let inserts: Vec<&[serde_json::Value]> = inserts.iter().map(|v| v.as_slice()).collect();
        self.execute(&inserts[..]).await
    }

    async fn insert_solution_into_pool(
        &self,
        solution: Signed<essential_types::solution::Solution>,
    ) -> anyhow::Result<()> {
        let hash = encode(&hash(&solution.data));
        let signature = encode_signature(&solution.signature);
        let solution = encode(&solution.data);

        let inserts = &[include_sql!(
            "insert/solutions_pool.sql",
            hash,
            solution,
            signature
        )];
        self.execute(&inserts[..]).await
    }

    async fn move_solutions_to_solved(
        &self,
        solutions: &[essential_types::Hash],
    ) -> anyhow::Result<()> {
        // Get the time this batch was created at.
        let created_at = std::time::SystemTime::now();
        let unix_time = created_at.duration_since(std::time::UNIX_EPOCH)?;

        // Encode the data into base64 blobs.
        let batch_hash = encode(&hash(&solutions));

        // For each solution, insert the solution into the solved table and delete from the pool.
        let inserts = solutions.iter().flat_map(|hash| {
            let hash = encode(hash);
            [
                include_sql!(owned "insert/copy_to_solved.sql", hash.clone()),
                include_sql!(owned "update/delete_from_solutions_pool.sql", hash),
            ]
        });

        // First insert the batch then the solutions.
        let mut sql = vec![include_sql!(
            owned "insert/batch.sql",
            batch_hash,
            unix_time.as_secs(),
            unix_time.subsec_nanos()
        )];
        sql.extend(inserts);

        // TODO: Is there a way to avoid this?
        // Maybe create an owned version of execute.
        let sql: Vec<&[serde_json::Value]> = sql.iter().map(|v| v.as_slice()).collect();
        self.execute(&sql[..]).await
    }

    async fn update_state(
        &self,
        address: &essential_types::ContentAddress,
        key: &essential_types::Key,
        value: Option<essential_types::Word>,
    ) -> anyhow::Result<Option<essential_types::Word>> {
        let address = encode(address);
        let key = encode(key);
        match value {
            Some(value) => {
                // Update the value and return the existing value if it exists.
                let inserts = &[
                    include_sql!("query/get_state.sql", address.clone(), key.clone()),
                    include_sql!("update/update_state.sql", key, value, address),
                ];
                self.execute_query_word(&inserts[..]).await
            }
            None => {
                // Delete the value and return the existing value if it exists.
                let inserts = &[
                    include_sql!("query/get_state.sql", address.clone(), key.clone()),
                    include_sql!("update/delete_state.sql", address, key),
                ];
                self.execute_query_word(&inserts[..]).await
            }
        }
    }

    async fn get_intent(
        &self,
        address: &essential_types::IntentAddress,
    ) -> anyhow::Result<Option<essential_types::intent::Intent>> {
        let intent_hash = encode(&address.intent);
        let sql = &[include_sql!("query/get_intent.sql", intent_hash)];
        let queries = self.query_values(sql).await?;

        // Expecting single query, single row, single column
        let Some(serde_json::Value::String(intent)) = single_value(queries) else {
            return Ok(None);
        };
        Ok(Some(decode(&intent)?))
    }

    async fn get_intent_set(
        &self,
        address: &essential_types::ContentAddress,
    ) -> anyhow::Result<Option<Signed<Vec<essential_types::intent::Intent>>>> {
        let address = encode(address);
        let sql = &[
            include_sql!("query/get_intent_set_signature.sql", address.clone()),
            include_sql!("query/get_intent_set.sql", address),
        ];
        let queries = self.query_values(sql).await?;
        values::get_intent_set(queries)
    }

    async fn list_intent_sets(
        &self,
        time_range: Option<std::ops::Range<std::time::Duration>>,
        page: Option<usize>,
    ) -> anyhow::Result<Vec<Vec<essential_types::intent::Intent>>> {
        let page = page.unwrap_or(0);
        let queries = match time_range {
            Some(range) => {
                let sql = &[include_sql!(
                    "query/list_intent_sets_by_time.sql",
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
                    include_sql!(named "query/list_intent_sets.sql", "page_size" => PAGE_SIZE, "page_number" => page),
                ];
                self.query_values(sql).await?
            }
        };
        values::list_intent_sets(queries)
    }

    async fn list_solutions_pool(
        &self,
    ) -> anyhow::Result<Vec<Signed<essential_types::solution::Solution>>> {
        // TODO: Maybe we want to page this?
        let sql = &[include_sql!("query/list_solutions_pool.sql")];
        let queries = self.query_values(sql).await?;
        values::list_solutions_pool(queries)
    }

    async fn list_winning_blocks(
        &self,
        time_range: Option<std::ops::Range<std::time::Duration>>,
        page: Option<usize>,
    ) -> anyhow::Result<Vec<Block>> {
        let page = page.unwrap_or(0);
        let queries = match time_range {
            Some(range) => {
                let sql = &[include_sql!(named "query/list_winning_batches.sql",
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

    async fn query_state(
        &self,
        address: &essential_types::ContentAddress,
        key: &essential_types::Key,
    ) -> anyhow::Result<Option<essential_types::Word>> {
        let address = encode(address);
        let key = encode(key);
        let sql = &[include_sql!("query/get_state.sql", address, key)];
        let queries = self.query_values(sql).await?;
        let r = single_value(queries).and_then(|v| match v {
            serde_json::Value::Number(v) => v.as_i64(),
            _ => None,
        });
        Ok(r)
    }

    async fn get_storage_layout(
        &self,
        address: &essential_types::ContentAddress,
    ) -> anyhow::Result<Option<StorageLayout>> {
        let address = encode(address);
        let sql = &[include_sql!("query/get_storage_layout.sql", address)];
        let queries = self.query_values(sql).await?;
        let r = single_value(queries).and_then(|v| match v {
            serde_json::Value::String(v) => decode(&v).ok(),
            _ => None,
        });
        Ok(r)
    }
}
