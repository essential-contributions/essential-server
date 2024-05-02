use super::*;
use crate::encode;
use serde_json::Number;
use storage::failed_solution::{CheckOutcome, SolutionFailReason};
use test_utils::{empty::Empty, sign_with_random_keypair};

#[test]
fn test_empty_query() {
    let queries = QueryValues {
        queries: vec![None],
    };

    assert_eq!(get_solution(queries).unwrap(), None);
}

#[test]
fn test_invalid_query() {
    let queries = QueryValues { queries: vec![] };

    get_solution(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![None, None],
    };

    get_solution(queries).unwrap_err();
}

#[test]
fn test_valid_solution() {
    let Signed { data, signature } = sign_with_random_keypair(Solution::empty());
    let reason = SolutionFailReason::ConstraintsFailed("test".to_string());
    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(encode(&data)),
                    Value::String(encode(&signature)),
                    Value::Number(1.into()),
                    Value::Null,
                ],
            }],
        })],
    };

    let r = get_solution(queries).unwrap().unwrap();
    let expected = SolutionOutcome {
        solution: Signed {
            data: data.clone(),
            signature: signature.clone(),
        },
        outcome: CheckOutcome::Success(1),
    };
    assert_eq!(r, expected);

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(encode(&data)),
                    Value::String(encode(&signature)),
                    Value::Null,
                    Value::String(encode(&reason)),
                ],
            }],
        })],
    };

    let r = get_solution(queries).unwrap().unwrap();
    let expected = SolutionOutcome {
        solution: Signed { data, signature },
        outcome: CheckOutcome::Fail(reason),
    };
    assert_eq!(r, expected);
}

#[test]
fn test_invalid_solution() {
    let Signed { data, signature } = sign_with_random_keypair(PartialSolution::empty());
    let reason = SolutionFailReason::ConstraintsFailed("test".to_string());
    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::Bool(true),
                    Value::String(encode(&signature)),
                    Value::Number(1.into()),
                    Value::String(encode(&reason)),
                ],
            }],
        })],
    };

    get_solution(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(encode(&data)),
                    Value::Bool(true),
                    Value::Number(1.into()),
                    Value::String(encode(&reason)),
                ],
            }],
        })],
    };

    get_solution(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(encode(&data)),
                    Value::String(encode(&signature)),
                    Value::Bool(true),
                    Value::String(encode(&reason)),
                ],
            }],
        })],
    };

    get_solution(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(encode(&data)),
                    Value::String(encode(&signature)),
                    Value::Number(1.into()),
                    Value::Bool(true),
                ],
            }],
        })],
    };

    get_solution(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(encode(&data)),
                    Value::String(encode(&signature)),
                    Value::Number(1.into()),
                    Value::String(encode(&reason)),
                ],
            }],
        })],
    };

    get_solution(queries).unwrap_err();
}

#[test]
fn test_invalid_data() {
    let invalid = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_string();
    let Signed { data, signature } = sign_with_random_keypair(PartialSolution::empty());
    let reason = SolutionFailReason::ConstraintsFailed("test".to_string());

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(invalid.clone()),
                    Value::String(encode(&signature)),
                    Value::Null,
                    Value::String(encode(&reason)),
                ],
            }],
        })],
    };
    get_solution(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(encode(&data)),
                    Value::String(invalid.clone()),
                    Value::Null,
                    Value::String(encode(&reason)),
                ],
            }],
        })],
    };
    get_solution(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(encode(&data)),
                    Value::String(encode(&signature)),
                    Value::Number(Number::from_f64(1.0).unwrap()),
                    Value::Null,
                ],
            }],
        })],
    };
    get_solution(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(encode(&data)),
                    Value::String(encode(&signature)),
                    Value::Null,
                    Value::String(invalid.clone()),
                ],
            }],
        })],
    };
    get_solution(queries).unwrap_err();
}

#[test]
fn test_wrong_num_columns() {
    let Signed { data, signature } = sign_with_random_keypair(PartialSolution::empty());
    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(encode(&data)),
                    Value::String(encode(&signature)),
                    Value::String(encode(&data)),
                ],
            }],
        })],
    };

    get_solution(queries).unwrap_err();
}

#[test]
fn test_wrong_num_rows() {
    let Signed { data, signature } = sign_with_random_keypair(PartialSolution::empty());
    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![
                Columns {
                    columns: vec![
                        Value::String(encode(&data)),
                        Value::String(encode(&signature)),
                    ],
                },
                Columns {
                    columns: vec![
                        Value::String(encode(&data)),
                        Value::String(encode(&signature)),
                    ],
                },
            ],
        })],
    };

    get_solution(queries).unwrap_err();
}
