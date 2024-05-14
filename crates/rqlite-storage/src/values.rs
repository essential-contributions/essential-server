use std::{collections::BTreeMap, time::Duration};

use anyhow::bail;
use essential_storage::failed_solution::{CheckOutcome, FailedSolution, SolutionOutcome};
use essential_types::{intent::Intent, solution::Solution, Batch, Block, Signature, Signed, Word};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{decode, RESULTS_KEY};

#[cfg(test)]
mod test_get_intent_set;
#[cfg(test)]
mod test_get_solution;
#[cfg(test)]
mod test_list_failed_solutions;
#[cfg(test)]
mod test_list_intent_sets;
#[cfg(test)]
mod test_list_solutions;
#[cfg(test)]
mod test_list_winning_blocks;
#[cfg(test)]
mod test_map_execute_to_word;
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

pub fn get_intent_set(queries: QueryValues) -> anyhow::Result<Option<Signed<Vec<Intent>>>> {
    // Expecting two results because we made two queries
    let (signature, intents) = match &queries.queries[..] {
        [Some(Rows { rows: signature }), Some(Rows { rows: intents })] => (signature, intents),
        [None, None] => return Ok(None),
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

    // Intents should only have a single row
    let [Columns { columns: intents }] = &intents[..] else {
        bail!("expected a single row");
    };

    // Decode the signature
    let signature: Signature = decode(signature)?;

    // Decode the intents
    let intents: Vec<Intent> = intents
        .iter()
        .map(|intent| match intent {
            serde_json::Value::String(intent) => decode(intent),
            _ => Err(anyhow::anyhow!("unexpected column type")),
        })
        .collect::<Result<_, _>>()?;

    Ok(Some(Signed {
        data: intents,
        signature,
    }))
}

pub fn get_solution(
    QueryValues { queries }: QueryValues,
) -> Result<Option<SolutionOutcome>, anyhow::Error> {
    let rows = match &queries[..] {
        [Some(Rows { rows })] => rows,
        [None] => return Ok(None),
        _ => bail!("expected a single query {:?}", queries),
    };

    let [Columns { columns }] = &rows[..] else {
        bail!("expected a single row");
    };

    match &columns[..] {
        [Value::String(solution), Value::String(signature), Value::Number(block_number), Value::Null] =>
        {
            let solution = decode(solution)?;
            let signature = decode(signature)?;
            let block_number = block_number
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("failed to parse block_number"))?;
            Ok(Some(SolutionOutcome {
                solution: Signed {
                    data: solution,
                    signature,
                },
                outcome: CheckOutcome::Success(block_number),
            }))
        }
        [Value::String(solution), Value::String(signature), Value::Null, Value::String(reason)] => {
            let solution = decode(solution)?;
            let signature = decode(signature)?;
            let reason = decode(reason)?;
            Ok(Some(SolutionOutcome {
                solution: Signed {
                    data: solution,
                    signature,
                },
                outcome: CheckOutcome::Fail(reason),
            }))
        }
        _ => bail!("unexpected columns: {:?}", columns),
    }
}

pub fn list_intent_sets(QueryValues { queries }: QueryValues) -> anyhow::Result<Vec<Vec<Intent>>> {
    // Only expecting a single query.
    let rows = match &queries[..] {
        [Some(Rows { rows })] => rows,
        [None] => return Ok(Vec::with_capacity(0)),
        _ => bail!("expected a single query {:?}", queries),
    };

    // If the query isn't empty there should be at least one row.
    if rows.is_empty() {
        bail!("expected at least one row")
    }

    // Expecting an intent per row with two columns.
    // The first column is the set_id and the second column is the intent.
    // The intents are grouped into their respective sets.
    //
    // TODO: The sql outputs the intents ordered by set_id, then by intent id.
    // Could we use this fact to avoid needing to sort them into a BTreeMap?
    let out = rows
        .iter()
        .try_fold(
            BTreeMap::<_, Vec<_>>::new(),
            |mut map, Columns { columns }| match &columns[..] {
                [serde_json::Value::Number(set_id), serde_json::Value::String(intent)] => {
                    match set_id.as_u64() {
                        Some(set_id) => {
                            let intent: Intent = decode(intent)?;
                            map.entry(set_id).or_default().push(intent);
                            Ok(map)
                        }
                        None => Err(anyhow::anyhow!("failed to parse set_id")),
                    }
                }
                _ => Err(anyhow::anyhow!("unexpected columns: {:?}", columns)),
            },
        )?
        // TODO: Is there a way to avoid this double iteration?
        .into_values()
        .collect();

    Ok(out)
}

pub fn list_solutions_pool(queries: QueryValues) -> anyhow::Result<Vec<Signed<Solution>>> {
    list_solutions(queries)
}

fn list_solutions<S>(QueryValues { queries }: QueryValues) -> anyhow::Result<Vec<Signed<S>>>
where
    S: DeserializeOwned,
{
    // Only expecting a single query.
    let rows = match &queries[..] {
        [Some(Rows { rows })] => rows,
        [None] => return Ok(Vec::with_capacity(0)),
        _ => bail!("expected a single query {:?}", queries),
    };

    // If the query isn't empty there should be at least one row.
    if rows.is_empty() {
        bail!("expected at least one row")
    }

    // Decode signature and solution from each row.
    rows.iter()
        .map(|Columns { columns }| match &columns[..] {
            [signature, solution] => {
                let signature = match signature {
                    serde_json::Value::String(signature) => decode(signature)?,
                    _ => bail!("unexpected column type {:?} for signature", signature),
                };
                let solution = match solution {
                    serde_json::Value::String(solution) => decode(solution)?,
                    _ => bail!("unexpected column type {:?} for solution", solution),
                };
                Ok(Signed {
                    data: solution,
                    signature,
                })
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
        [None] => return Ok(Vec::with_capacity(0)),
        _ => bail!("expected a single query {:?}", queries),
    };

    // If the query isn't empty there should be at least one row.
    if rows.is_empty() {
        bail!("expected at least one row")
    }

    // Decode signature and solution from each row.
    rows.iter()
        .map(|Columns { columns }| match &columns[..] {
            [signature, solution, reason] => {
                let signature = match signature {
                    serde_json::Value::String(signature) => decode(signature)?,
                    _ => bail!("unexpected column type {:?} for signature", signature),
                };
                let solution = match solution {
                    serde_json::Value::String(solution) => decode(solution)?,
                    _ => bail!("unexpected column type {:?} for solution", solution),
                };
                let reason = match reason {
                    serde_json::Value::String(reason) => decode(reason)?,
                    _ => bail!("unexpected column type {:?} for reason", reason),
                };
                Ok(FailedSolution {
                    solution: Signed {
                        data: solution,
                        signature,
                    },
                    reason,
                })
            }
            _ => Err(anyhow::anyhow!("unexpected columns: {:?}", columns)),
        })
        .collect()
}

pub fn list_winning_blocks(QueryValues { queries }: QueryValues) -> anyhow::Result<Vec<Block>> {
    // Only expecting a single query.
    let rows = match &queries[..] {
        [Some(Rows { rows })] => rows,
        [None] => return Ok(Vec::with_capacity(0)),
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
        [Value::Number(batch_id), Value::String(solution), Value::String(signature), Value::Number(created_at_secs), Value::Number(created_at_nanos)] => {
            match (
                batch_id.as_u64(),
                created_at_secs.as_u64(),
                created_at_nanos.as_u64(),
            ) {
                (Some(batch_id), Some(created_at_secs), Some(created_at_nanos)) => {
                    let solution = decode(solution)?;
                    let signature = decode(signature)?;
                    let Some(number) = batch_id.checked_sub(1) else {
                        bail!("batch_id must be greater than 0");
                    };
                    map.entry(batch_id)
                        .or_insert_with(|| Block {
                            number,
                            timestamp: Duration::new(created_at_secs, created_at_nanos as u32),
                            batch: Batch {
                                solutions: Vec::with_capacity(1),
                            },
                        })
                        .batch
                        .solutions
                        .push(Signed {
                            data: solution,
                            signature,
                        });
                    Ok(map)
                }
                _ => bail!("Failed to parse batch_id, created_at_secs, or created_at_nanos"),
            }
        }
        _ => bail!("unexpected columns: {:?}", columns),
    }
}

pub fn map_execute_to_word(
    mut value: serde_json::Map<String, serde_json::Value>,
) -> anyhow::Result<Option<Word>> {
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
        return Ok(None);
    };

    // There must be a single row which is an array
    let [serde_json::Value::Array(columns)] = &rows[..] else {
        bail!("expected a single row");
    };

    // There must be a single column which is a number
    let [serde_json::Value::Number(word)] = &columns[..] else {
        bail!("expected a single column");
    };

    // Parse the word
    let Some(word) = word.as_i64() else {
        bail!("failed to parse word");
    };

    Ok(Some(word))
}

/// Map the execute query results to words.
///
/// Note this is designed for queries where the first query is a read query
/// then it alternates between write and read queries.
pub fn map_execute_to_words(QueryValues { queries }: QueryValues) -> Vec<Option<Word>> {
    queries
        .into_iter()
        .enumerate()
        // Skip every second result as they are the results of the write queries
        // and not the read queries that we are interested in.
        .filter(|(i, _)| i % 2 == 0)
        .map(|(_, row)| {
            row.and_then(|Rows { rows }| {
                let col = rows.into_iter().next()?.columns.into_iter().next()?;
                match col {
                    Value::Number(n) => n.as_i64(),
                    _ => None,
                }
            })
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
    let queries = queries.unwrap_or_else(|| Vec::with_capacity(0));
    Ok(QueryValues { queries })
}
