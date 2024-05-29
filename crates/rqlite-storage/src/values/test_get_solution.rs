use super::*;
use crate::encode;
use essential_storage::failed_solution::{CheckOutcome, SolutionFailReason};
use serde_json::Number;
use test_utils::empty::Empty;

#[test]
fn test_empty_query() {
    let queries = QueryValues {
        queries: vec![None, None],
    };

    assert_eq!(get_solution(queries).unwrap(), None);
}

#[test]
fn test_invalid_query() {
    let queries = QueryValues { queries: vec![] };

    get_solution(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![None, None, None],
    };

    get_solution(queries).unwrap_err();
}

#[test]
fn test_valid_solution() {
    let solution = Solution::empty();
    let reason = SolutionFailReason::ConstraintsFailed("test".to_string());
    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&solution))],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::Number(1.into()), Value::Null],
                }],
            }),
        ],
    };

    let r = get_solution(queries).unwrap().unwrap();
    let expected = SolutionOutcome {
        solution: solution.clone(),
        outcome: vec![CheckOutcome::Success(1)],
    };
    assert_eq!(r, expected);

    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&solution))],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::Null, Value::String(encode(&reason))],
                }],
            }),
        ],
    };

    let r = get_solution(queries).unwrap().unwrap();
    let expected = SolutionOutcome {
        solution,
        outcome: vec![CheckOutcome::Fail(reason)],
    };
    assert_eq!(r, expected);
}

#[test]
fn test_invalid_solution() {
    let solution = Solution::empty();
    let reason = SolutionFailReason::ConstraintsFailed("test".to_string());
    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::Bool(true)],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::Number(1.into()), Value::String(encode(&reason))],
                }],
            }),
        ],
    };

    get_solution(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&solution)), Value::Bool(true)],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::Number(1.into()), Value::String(encode(&reason))],
                }],
            }),
        ],
    };

    get_solution(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&solution))],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![
                        Value::Bool(true),
                        Value::Number(1.into()),
                        Value::String(encode(&reason)),
                    ],
                }],
            }),
        ],
    };

    get_solution(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&solution))],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::Bool(true), Value::String(encode(&reason))],
                }],
            }),
        ],
    };

    get_solution(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&solution))],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::Number(1.into()), Value::Bool(true)],
                }],
            }),
        ],
    };

    get_solution(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&solution))],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::Number(1.into()), Value::String(encode(&reason))],
                }],
            }),
        ],
    };

    get_solution(queries).unwrap_err();
}

#[test]
fn test_invalid_data() {
    let invalid = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_string();
    let solution = Solution::empty();
    let reason = SolutionFailReason::ConstraintsFailed("test".to_string());

    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(invalid.clone())],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::Null, Value::String(encode(&reason))],
                }],
            }),
        ],
    };
    get_solution(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![
                        Value::String(encode(&solution)),
                        Value::String(invalid.clone()),
                    ],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::Null, Value::String(encode(&reason))],
                }],
            }),
        ],
    };
    get_solution(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&solution))],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::Number(Number::from_f64(1.0).unwrap()), Value::Null],
                }],
            }),
        ],
    };
    get_solution(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&solution))],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::Null, Value::String(invalid.clone())],
                }],
            }),
        ],
    };
    get_solution(queries).unwrap_err();
}

#[test]
fn test_wrong_num_columns() {
    let solution = Solution::empty();
    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(encode(&solution)),
                    Value::String(encode(&solution)),
                ],
            }],
        })],
    };

    get_solution(queries).unwrap_err();
}

#[test]
fn test_wrong_num_rows() {
    let solution = Solution::empty();
    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![
                Columns {
                    columns: vec![Value::String(encode(&solution))],
                },
                Columns {
                    columns: vec![Value::String(encode(&solution))],
                },
            ],
        })],
    };

    get_solution(queries).unwrap_err();
}
