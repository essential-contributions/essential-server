use test_utils::{empty_partial_solution, sign_with_random_keypair};

use crate::encode;

use super::*;

#[test]
fn test_empty_query() {
    let queries = QueryValues {
        queries: vec![None],
    };

    assert_eq!(get_partial_solution(queries).unwrap(), None);
}

#[test]
fn test_invalid_query() {
    let queries = QueryValues { queries: vec![] };

    get_partial_solution(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![None, None],
    };

    get_partial_solution(queries).unwrap_err();
}

#[test]
fn test_valid_partial() {
    let Signed { data, signature } = sign_with_random_keypair(empty_partial_solution());
    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(encode(&data)),
                    Value::String(encode(&signature)),
                ],
            }],
        })],
    };

    let r = get_partial_solution(queries).unwrap().unwrap();
    let expected = Signed { data, signature };
    assert_eq!(r, expected);
}

#[test]
fn test_invalid_partial() {
    let Signed { data: _, signature } = sign_with_random_keypair(empty_partial_solution());
    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![Value::Bool(true), Value::String(encode(&signature))],
            }],
        })],
    };

    get_partial_solution(queries).unwrap_err();
}

#[test]
fn test_invalid_sig() {
    let Signed { data, signature: _ } = sign_with_random_keypair(empty_partial_solution());
    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![Value::String(encode(&data)), Value::Bool(true)],
            }],
        })],
    };

    get_partial_solution(queries).unwrap_err();
}

#[test]
fn test_invalid_data() {
    let invalid = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_string();
    let Signed { data, signature } = sign_with_random_keypair(empty_partial_solution());

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(invalid.clone()),
                    Value::String(encode(&signature)),
                ],
            }],
        })],
    };
    get_partial_solution(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![Value::String(encode(&data)), Value::String(invalid)],
            }],
        })],
    };
    get_partial_solution(queries).unwrap_err();
}

#[test]
fn test_wrong_num_columns() {
    let Signed { data, signature } = sign_with_random_keypair(empty_partial_solution());
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

    get_partial_solution(queries).unwrap_err();
}

#[test]
fn test_wrong_num_rows() {
    let Signed { data, signature } = sign_with_random_keypair(empty_partial_solution());
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

    get_partial_solution(queries).unwrap_err();
}
