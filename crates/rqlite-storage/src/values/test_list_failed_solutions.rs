use super::*;
use crate::encode;
use essential_storage::failed_solution::SolutionFailReason;
use test_utils::empty::Empty;

#[test]
fn test_empty_query() {
    let queries = QueryValues {
        queries: vec![None],
    };

    assert!(list_failed_solutions(queries).unwrap().is_empty());
}

#[test]
fn test_invalid_query() {
    let queries = QueryValues { queries: vec![] };

    list_failed_solutions(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![None, None],
    };

    list_failed_solutions(queries).unwrap_err();
}

#[test]
fn test_valid_query() {
    let solution = Solution::empty();
    let reason = SolutionFailReason::ConstraintsFailed("test".to_string());
    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(encode(&solution)),
                    Value::String(encode(&reason)),
                ],
            }],
        })],
    };

    let r = list_failed_solutions(queries).unwrap();
    let expected = vec![FailedSolution {
        solution: solution.clone(),
        reason: reason.clone(),
    }];
    assert_eq!(r, expected);

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![
                Columns {
                    columns: vec![
                        Value::String(encode(&solution)),
                        Value::String(encode(&reason)),
                    ],
                },
                Columns {
                    columns: vec![
                        Value::String(encode(&solution)),
                        Value::String(encode(&reason)),
                    ],
                },
            ],
        })],
    };

    let r = list_failed_solutions(queries).unwrap();
    let expected = vec![
        FailedSolution {
            solution: solution.clone(),
            reason: reason.clone(),
        },
        FailedSolution { solution, reason },
    ];
    assert_eq!(r, expected);
}

#[test]
fn test_invalid_data() {
    let invalid = "xxxxxxx".to_string();
    let solution = Solution::empty();
    let reason = SolutionFailReason::ConstraintsFailed("test".to_string());

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(invalid.clone()),
                    Value::String(encode(&solution)),
                    Value::String(encode(&reason)),
                ],
            }],
        })],
    };
    list_failed_solutions(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(invalid.clone()),
                    Value::String(encode(&reason)),
                ],
            }],
        })],
    };
    list_failed_solutions(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(encode(&solution)),
                    Value::String(invalid.clone()),
                ],
            }],
        })],
    };
    list_failed_solutions(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::Bool(true),
                    Value::String(encode(&solution)),
                    Value::String(encode(&reason)),
                ],
            }],
        })],
    };
    list_failed_solutions(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![Value::Bool(true), Value::String(encode(&reason))],
            }],
        })],
    };
    list_failed_solutions(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![Value::String(encode(&solution)), Value::Bool(true)],
            }],
        })],
    };
    list_failed_solutions(queries).unwrap_err();
}

#[test]
fn test_wrong_num_columns() {
    let solution = Solution::empty();
    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![Value::String(encode(&solution))],
            }],
        })],
    };
    list_failed_solutions(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![Value::String(encode(&solution))],
            }],
        })],
    };
    list_failed_solutions(queries).unwrap_err();
}

#[test]
fn test_wrong_num_rows() {
    let queries = QueryValues {
        queries: vec![Some(Rows { rows: vec![] })],
    };
    list_failed_solutions(queries).unwrap_err();
}
