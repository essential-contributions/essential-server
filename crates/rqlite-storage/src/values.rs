use std::{collections::BTreeMap, time::Duration};

use anyhow::bail;
use essential_types::{
    intent::Intent,
    solution::{PartialSolution, Solution},
    Batch, Block, Signature, Signed, Word,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{decode, RESULTS_KEY};

#[derive(Debug)]
pub struct QueryValues {
    pub queries: Vec<Option<Rows>>,
}

#[derive(Debug)]
pub struct Columns {
    pub columns: Vec<Value>,
}

#[derive(Debug)]
pub struct Rows {
    pub rows: Vec<Columns>,
}

/// Get a single value from the query results.
pub fn single_value(queries: QueryValues) -> Option<Value> {
    queries
        .queries
        .into_iter()
        .next()
        .flatten()
        .and_then(|rows| rows.rows.into_iter().next())
        .and_then(|columns| columns.columns.into_iter().next())
}

pub fn get_intent_set(
    QueryValues { queries }: QueryValues,
) -> anyhow::Result<Option<Signed<Vec<Intent>>>> {
    // Expecting two results because we made two queries
    let mut queries = queries.into_iter();
    let (Some(serde_json::Value::String(signature)), Some(intents)) = (
        // Expecting only a single row and single column for signature.
        queries
            .next()
            .flatten()
            .and_then(|Rows { rows }| rows.into_iter().next())
            .and_then(|Columns { columns }| columns.into_iter().next()),
        // Expecting only a single row and multiple columns for intents.
        queries
            .next()
            .flatten()
            .and_then(|Rows { rows }| rows.into_iter().next())
            .map(|Columns { columns }| columns),
    ) else {
        return Ok(None);
    };

    let signature: Signature = decode(&signature)?;

    // Decode the intents
    let intents: Vec<Intent> = intents
        .into_iter()
        .filter_map(|intent| match intent {
            serde_json::Value::String(intent) => Some(decode(&intent)),
            _ => None,
        })
        .collect::<Result<_, _>>()?;

    Ok(Some(Signed {
        data: intents,
        signature,
    }))
}

pub fn get_partial_solution(
    QueryValues { queries }: QueryValues,
) -> Result<Option<Signed<PartialSolution>>, anyhow::Error> {
    let Some(Columns { columns }) = queries
        .into_iter()
        .next()
        .flatten()
        .and_then(|rows| rows.rows.into_iter().next())
    else {
        return Ok(None);
    };

    match &columns[..] {
        [Value::String(solution), Value::String(signature)] => {
            let solution = decode(solution)?;
            let signature = decode(signature)?;
            Ok(Some(Signed {
                data: solution,
                signature,
            }))
        }
        _ => bail!("unexpected columns: {:?}", columns),
    }
}

pub fn list_intent_sets(QueryValues { queries }: QueryValues) -> anyhow::Result<Vec<Vec<Intent>>> {
    // Only expecting a single query because
    // we only made a single query
    let out = queries
        .into_iter()
        .next()
        .flatten()
        // Expecting only a multiple rows and multiple columns for intents.
        .map(|Rows { rows }| {
            rows.into_iter()
                .fold(
                    BTreeMap::<_, Vec<_>>::new(),
                    |mut map, Columns { mut columns }| {
                        if let (
                            Some(serde_json::Value::String(intent)),
                            Some(serde_json::Value::Number(set_id)),
                        ) = (columns.pop(), columns.pop())
                        {
                            if let Some(set_id) = set_id.as_u64() {
                                let intent: Option<Intent> = decode(&intent).ok();
                                if let Some(intent) = intent {
                                    map.entry(set_id).or_default().push(intent);
                                }
                            }
                        }
                        map
                    },
                )
                .into_values()
                .collect()
        })
        .unwrap_or_else(|| Vec::with_capacity(0));

    Ok(out)
}

pub fn list_solutions_pool(queries: QueryValues) -> anyhow::Result<Vec<Signed<Solution>>> {
    list_solutions(queries)
}

pub fn list_partial_solutions_pool(
    queries: QueryValues,
) -> Result<Vec<Signed<PartialSolution>>, anyhow::Error> {
    list_solutions(queries)
}

fn list_solutions<S>(QueryValues { queries }: QueryValues) -> anyhow::Result<Vec<Signed<S>>>
where
    S: DeserializeOwned,
{
    let r = queries
        .into_iter()
        .next()
        .flatten()
        .map(|Rows { rows }| {
            rows.into_iter()
                .filter_map(|Columns { columns }| match &columns[..] {
                    [signature, solution] => {
                        let solution = match solution {
                            serde_json::Value::String(solution) => decode(solution).ok(),
                            _ => None,
                        };
                        let signature = match signature {
                            serde_json::Value::String(signature) => decode(signature).ok(),
                            _ => None,
                        };
                        Some(Signed {
                            data: solution?,
                            signature: signature?,
                        })
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_else(|| Vec::with_capacity(0));
    Ok(r)
}

pub fn list_winning_blocks(QueryValues { queries }: QueryValues) -> anyhow::Result<Vec<Block>> {
    let Some(Rows { rows }) = queries.into_iter().next().flatten() else {
        return Ok(Vec::with_capacity(0));
    };
    let r = rows
        .into_iter()
        .try_fold(BTreeMap::new(), |map, Columns { columns }| {
            map_solution_to_block(map, &columns)
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
                    map.entry(batch_id)
                        .or_insert_with(|| Block {
                            number: batch_id - 1,
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
    let value = value
        .remove(RESULTS_KEY)
        .and_then(|r| match r {
            serde_json::Value::Array(a) => a.into_iter().next(),
            _ => None,
        })
        .and_then(|r| match r {
            serde_json::Value::Object(mut o) => o.remove("values"),
            _ => None,
        })
        .and_then(|r| match r {
            serde_json::Value::Array(a) => match a.into_iter().next()? {
                serde_json::Value::Array(a) => match a.into_iter().next()? {
                    serde_json::Value::Number(n) => n.as_i64(),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        });
    Ok(value)
}

pub fn map_execute_to_words(QueryValues { queries }: QueryValues) -> Vec<Option<Word>> {
    queries
        .into_iter()
        .enumerate()
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
    let queries = value
        .remove(RESULTS_KEY)
        .and_then(|r| match r {
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
        })
        .unwrap_or_else(|| Vec::with_capacity(0));
    Ok(QueryValues { queries })
}
