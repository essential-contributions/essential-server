use std::{
    collections::{BTreeMap, HashMap},
    time::Duration,
};

use anyhow::{bail, ensure};
use essential_storage::failed_solution::{CheckOutcome, FailedSolution, SolutionOutcomes};
use essential_types::{
    contract::{Contract, SignedContract},
    predicate::Predicate,
    solution::Solution,
    Block, Hash, Signature, Word,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{decode, RESULTS_KEY};

#[cfg(test)]
mod test_get_contract;
#[cfg(test)]
mod test_get_solution;
#[cfg(test)]
mod test_list_contracts;
#[cfg(test)]
mod test_list_failed_solutions;
#[cfg(test)]
mod test_list_solutions;
#[cfg(test)]
mod test_list_winning_blocks;
#[cfg(test)]
mod test_map_execute_to_values;
#[cfg(test)]
mod test_map_query_to_query_values;
#[cfg(test)]
mod test_map_solution_to_block;
#[cfg(test)]
mod test_single_value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryValues {
    pub queries: Vec<Option<Rows>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Columns {
    pub columns: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rows {
    pub rows: Vec<Columns>,
}

/// Get a single value from the query results.
pub fn single_value(queries: &QueryValues) -> Option<&Value> {
    if let [Some(Rows { rows })] = &queries.queries[..] {
        if let [Columns { columns }] = &rows[..] {
            if let [value] = &columns[..] {
                return Some(value);
            }
        }
    }

    None
}

pub fn get_contract(queries: QueryValues) -> anyhow::Result<Option<SignedContract>> {
    // Expecting three results because we made two queries
    let (signature, salt, contract) = match &queries.queries[..] {
        [Some(Rows { rows: signature }), Some(Rows { rows: salt }), Some(Rows { rows: contract })] => {
            (signature, salt, contract)
        }
        [None, None, None] => return Ok(None),
        _ => bail!("expected two queries {:?}", queries),
    };

    // Signature should only have a single row
    let [Columns { columns: signature }] = &signature[..] else {
        bail!("expected a single row");
    };

    // Signature should only have a single column
    let [Value::String(signature)] = &signature[..] else {
        bail!("expected a single column");
    };

    // Salt should only have a single row
    let [Columns { columns: salt }] = &salt[..] else {
        bail!("expected a single row for salt");
    };

    // Salt should only have a single column
    let [Value::String(salt)] = &salt[..] else {
        bail!("expected a single column for salt");
    };

    // Decode the signature
    let signature: Signature = decode(signature)?;

    // Decode the salt
    let salt: Hash = decode(salt)?;

    // Decode the predicates
    let predicates: Vec<Predicate> = contract
        .iter()
        .map(|Columns { columns }| {
            let [predicate] = &columns[..] else {
                bail!("expected a single column");
            };
            match predicate {
                serde_json::Value::String(predicate) => decode(predicate),
                _ => Err(anyhow::anyhow!("unexpected column type")),
            }
        })
        .collect::<Result<_, _>>()?;

    Ok(Some(SignedContract {
        contract: Contract { predicates, salt },
        signature,
    }))
}

pub fn get_solution(
    QueryValues { queries }: QueryValues,
) -> Result<Option<SolutionOutcomes>, anyhow::Error> {
    let empty = Vec::new();

    let (solution, outcomes) = match &queries[..] {
        [Some(Rows { rows: solution }), Some(Rows { rows: outcomes })] => (solution, outcomes),
        [Some(Rows { rows: solution }), None] => (solution, &empty),
        [None, _] => return Ok(None),
        _ => bail!("expected two queries {:?}", queries),
    };

    let [Columns { columns: solution }] = &solution[..] else {
        bail!("expected a single row");
    };

    let [Value::String(solution)] = &solution[..] else {
        bail!("expected a single column");
    };

    let outcomes = outcomes
        .iter()
        .map(|Columns { columns }| match &columns[..] {
            [Value::Number(block_number), Value::Null, _, _] => block_number
                .as_u64()
                .and_then(|n| n.checked_sub(1))
                .map(CheckOutcome::Success)
                .ok_or_else(|| anyhow::anyhow!("failed to parse block_number")),
            [Value::Null, Value::String(reason), _, _] => decode(reason).map(CheckOutcome::Fail),
            _ => bail!("unexpected columns: {:?}", columns),
        })
        .collect::<anyhow::Result<_>>()?;

    let solution = decode(solution)?;

    Ok(Some(SolutionOutcomes {
        solution,
        outcome: outcomes,
    }))
}

pub fn list_contracts(QueryValues { queries }: QueryValues) -> anyhow::Result<Vec<Contract>> {
    // Only expecting two queries.
    let (salts, predicates) = match &queries[..] {
        [Some(Rows { rows: salts }), Some(Rows { rows: predicates })] => (salts, predicates),
        [None, None] => return Ok(Vec::new()),
        _ => bail!("expected a single query {:?}", queries),
    };

    // If the query isn't empty there should be at least one row.
    if salts.is_empty() || predicates.is_empty() {
        bail!("expected at least one row")
    }

    let salts =
        salts.iter().try_fold(
            HashMap::<_, _>::new(),
            |mut map, Columns { columns }| match &columns[..] {
                [serde_json::Value::Number(contract_id), serde_json::Value::String(salt)] => {
                    match contract_id.as_u64() {
                        Some(contract_id) => {
                            let salt: Hash = decode(salt)?;
                            if map.insert(contract_id, salt).is_none() {
                                Ok(map)
                            } else {
                                Err(anyhow::anyhow!("duplicate contract_id for salt"))
                            }
                        }
                        None => Err(anyhow::anyhow!("failed to parse contract_id")),
                    }
                }
                _ => Err(anyhow::anyhow!("unexpected columns: {:?}", columns)),
            },
        )?;

    // Expecting a predicate per row with two columns.
    // The first column is the contract_id and the second column is the predicate.
    // The contract are grouped into their respective contracts.
    //
    // TODO: The sql outputs the contract ordered by contract_id, then by predicate id.
    // Could we use this fact to avoid needing to sort them into a BTreeMap?
    let contracts = predicates
        .iter()
        .try_fold(
            BTreeMap::<_, Vec<_>>::new(),
            |mut map, Columns { columns }| match &columns[..] {
                [serde_json::Value::Number(contract_id), serde_json::Value::String(predicate)] => {
                    match contract_id.as_u64() {
                        Some(contract_id) => {
                            let predicate: Predicate = decode(predicate)?;
                            map.entry(contract_id).or_default().push(predicate);
                            Ok(map)
                        }
                        None => Err(anyhow::anyhow!("failed to parse contract_id")),
                    }
                }
                _ => Err(anyhow::anyhow!("unexpected columns: {:?}", columns)),
            },
        )?
        // // TODO: Is there a way to avoid this double iteration?
        .into_iter()
        .map(|(contract_id, predicates)| {
            let salt = salts
                .get(&contract_id)
                .ok_or_else(|| anyhow::anyhow!("missing salt for contract_id"))?;
            Ok(Contract {
                salt: *salt,
                predicates,
            })
        })
        .collect::<anyhow::Result<_>>()?;

    Ok(contracts)
}

pub fn list_solutions_pool(queries: QueryValues) -> anyhow::Result<Vec<Solution>> {
    list_solutions(queries)
}

fn list_solutions<S>(QueryValues { queries }: QueryValues) -> anyhow::Result<Vec<S>>
where
    S: DeserializeOwned,
{
    // Only expecting a single query.
    let rows = match &queries[..] {
        [Some(Rows { rows })] => rows,
        [None] => return Ok(Vec::new()),
        _ => bail!("expected a single query {:?}", queries),
    };

    // If the query isn't empty there should be at least one row.
    if rows.is_empty() {
        bail!("expected at least one row")
    }

    // Decode solution from each row.
    rows.iter()
        .map(|Columns { columns }| match &columns[..] {
            [solution] => {
                let solution = match solution {
                    serde_json::Value::String(solution) => decode(solution)?,
                    _ => bail!("unexpected column type {:?} for solution", solution),
                };
                Ok(solution)
            }
            _ => Err(anyhow::anyhow!("unexpected columns: {:?}", columns)),
        })
        .collect()
}

pub fn list_failed_solutions(
    QueryValues { queries }: QueryValues,
) -> anyhow::Result<Vec<FailedSolution>> {
    // Only expecting a single query.
    let rows = match &queries[..] {
        [Some(Rows { rows })] => rows,
        [None] => return Ok(Vec::new()),
        _ => bail!("expected a single query {:?}", queries),
    };

    // If the query isn't empty there should be at least one row.
    if rows.is_empty() {
        bail!("expected at least one row")
    }

    // Decode solution from each row.
    rows.iter()
        .map(|Columns { columns }| match &columns[..] {
            [solution, reason] => {
                let solution = match solution {
                    serde_json::Value::String(solution) => decode(solution)?,
                    _ => bail!("unexpected column type {:?} for solution", solution),
                };
                let reason = match reason {
                    serde_json::Value::String(reason) => decode(reason)?,
                    _ => bail!("unexpected column type {:?} for reason", reason),
                };
                Ok(FailedSolution { solution, reason })
            }
            _ => Err(anyhow::anyhow!("unexpected columns: {:?}", columns)),
        })
        .collect()
}

pub fn list_blocks(QueryValues { queries }: QueryValues) -> anyhow::Result<Vec<Block>> {
    // Only expecting a single query.
    let rows = match &queries[..] {
        [Some(Rows { rows })] => rows,
        [None] => return Ok(Vec::new()),
        _ => bail!("expected a single query {:?}", queries),
    };

    // If the query isn't empty there should be at least one row.
    if rows.is_empty() {
        bail!("expected at least one row")
    }

    // Map the rows to blocks.
    //
    // TODO: The sql outputs the blocks ordered by batch_id.
    // Could we use this fact to avoid needing to sort them into a BTreeMap?
    let r = rows
        .iter()
        .try_fold(BTreeMap::new(), |map, Columns { columns }| {
            map_solution_to_block(map, columns)
        });
    Ok(r?.into_values().collect())
}

fn map_solution_to_block(
    mut map: BTreeMap<u64, Block>,
    columns: &[Value],
) -> anyhow::Result<BTreeMap<u64, Block>> {
    match columns {
        [Value::Number(batch_id), Value::String(solution), Value::Number(created_at_secs), Value::Number(created_at_nanos)] => {
            match (
                batch_id.as_u64(),
                created_at_secs.as_u64(),
                created_at_nanos.as_u64(),
            ) {
                (Some(batch_id), Some(created_at_secs), Some(created_at_nanos)) => {
                    let solution = decode(solution)?;
                    let Some(number) = batch_id.checked_sub(1) else {
                        bail!("batch_id must be greater than 0");
                    };
                    map.entry(batch_id)
                        .or_insert_with(|| Block {
                            number,
                            timestamp: Duration::new(created_at_secs, created_at_nanos as u32),
                            solutions: Vec::new(),
                        })
                        .solutions
                        .push(solution);
                    Ok(map)
                }
                _ => bail!("Failed to parse batch_id, created_at_secs, or created_at_nanos"),
            }
        }
        _ => bail!("unexpected columns: {:?}", columns),
    }
}

pub fn map_execute_to_values(
    mut value: serde_json::Map<String, serde_json::Value>,
) -> anyhow::Result<Vec<Word>> {
    // Must have results key
    let Some(results) = value.remove(RESULTS_KEY) else {
        bail!("Query results are invalid");
    };

    // Results must be an array
    let serde_json::Value::Array(results) = results else {
        bail!("Query results are invalid");
    };

    // Results must have a single object
    let [serde_json::Value::Object(results), _] = &results[..] else {
        bail!("invalid amount of results");
    };

    // If the results doesn't contain the values key, return None
    let Some(serde_json::Value::Array(rows)) = results.get("values") else {
        return Ok(Vec::new());
    };

    // There must be a single row which is an array
    let [serde_json::Value::Array(columns)] = &rows[..] else {
        bail!("expected a single row");
    };

    // There must be a single column which is a blob
    let [serde_json::Value::String(words)] = &columns[..] else {
        bail!("expected a single column");
    };

    // Parse the words
    let words: Vec<Word> = decode(words)?;

    Ok(words)
}

/// Map the execute query results to values.
///
/// Note this is designed for queries where the first query is a read query
/// then it alternates between write and read queries.
pub fn map_execute_to_multiple_values(
    QueryValues { queries }: QueryValues,
) -> anyhow::Result<Vec<Vec<Word>>> {
    queries
        .into_iter()
        .enumerate()
        // Skip every second result as they are the results of the write queries
        // and not the read queries that we are interested in.
        .filter(|(i, _)| i % 2 == 0)
        .map(|(_, row)| {
            // If the row is None, return an empty vec
            let Some(Rows { rows }) = row else {
                return Ok(Vec::new());
            };
            let [Columns { columns }] = &rows[..] else {
                bail!("expected a single column per value")
            };
            let [Value::String(words)] = &columns[..] else {
                bail!("expected a single value per column")
            };
            decode(words)
        })
        .collect()
}

pub fn map_query_to_query_values(
    mut value: serde_json::Map<String, serde_json::Value>,
) -> anyhow::Result<QueryValues> {
    // Must have results key
    let Some(results) = value.remove(RESULTS_KEY) else {
        bail!("Query results are invalid");
    };

    let queries = match results {
        serde_json::Value::Array(queries) => {
            let queries = queries
                .into_iter()
                .map(|r| match r {
                    serde_json::Value::Object(mut o) => o.remove("values"),
                    _ => None,
                })
                .map(|rows| match rows {
                    Some(serde_json::Value::Array(rows)) => {
                        let rows = rows
                            .into_iter()
                            .filter_map(|columns| match columns {
                                serde_json::Value::Array(columns) => Some(Columns { columns }),
                                _ => None,
                            })
                            .collect();
                        Some(Rows { rows })
                    }
                    _ => None,
                })
                .collect();
            Some(queries)
        }
        _ => None,
    };
    let queries = queries.unwrap_or_default();
    Ok(QueryValues { queries })
}

pub fn assert_row_changed(
    result: &serde_json::Map<String, serde_json::Value>,
    sql: &[&[serde_json::Value]],
) -> anyhow::Result<()> {
    let Some(results) = result.get(RESULTS_KEY) else {
        bail!("Query results are invalid");
    };

    // Results must be an array
    let serde_json::Value::Array(results) = results else {
        bail!("Query results are invalid");
    };

    // Results must be a two object
    let [_, serde_json::Value::Object(results)] = &results[..] else {
        bail!("invalid amount of results");
    };

    ensure!(
        results.get("rows_affected") == Some(&serde_json::Value::Number(1.into())),
        "expected 1 row to be changed"
    );
    Ok(())
}
