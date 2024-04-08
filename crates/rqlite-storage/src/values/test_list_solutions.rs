use super::*;
use crate::encode;
use test_utils::{empty::Empty, sign_with_random_keypair};

#[test]
fn test_empty_query() {
    let queries = QueryValues {
        queries: vec![None],
    };

    assert!(list_solutions::<bool>(queries).unwrap().is_empty());
}

#[test]
fn test_invalid_query() {
    let queries = QueryValues { queries: vec![] };

    list_solutions::<bool>(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![None, None],
    };

    list_solutions::<bool>(queries).unwrap_err();
}

#[test]
fn test_valid_query() {
    let Signed { data, signature } = sign_with_random_keypair(true);
    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(encode(&signature)),
                    Value::String(encode(&data)),
                ],
            }],
        })],
    };

    let r = list_solutions::<bool>(queries).unwrap();
    let expected = vec![Signed {
        data,
        signature: signature.clone(),
    }];
    assert_eq!(r, expected);

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![
                Columns {
                    columns: vec![
                        Value::String(encode(&signature)),
                        Value::String(encode(&data)),
                    ],
                },
                Columns {
                    columns: vec![
                        Value::String(encode(&signature)),
                        Value::String(encode(&data)),
                    ],
                },
            ],
        })],
    };

    let r = list_solutions::<bool>(queries).unwrap();
    let expected = vec![
        Signed {
            data,
            signature: signature.clone(),
        },
        Signed { data, signature },
    ];
    assert_eq!(r, expected);
}

#[test]
fn test_invalid_data() {
    let invalid = "xxxxxxx".to_string();
    let Signed { data, signature } = sign_with_random_keypair(Solution::empty());

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![Value::String(invalid.clone()), Value::String(encode(&data))],
            }],
        })],
    };
    list_solutions::<Solution>(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(encode(&signature)),
                    Value::String(invalid.clone()),
                ],
            }],
        })],
    };
    list_solutions::<Solution>(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![Value::Bool(true), Value::String(encode(&data))],
            }],
        })],
    };
    list_solutions::<Solution>(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![Value::String(encode(&signature)), Value::Bool(true)],
            }],
        })],
    };
    list_solutions::<Solution>(queries).unwrap_err();
}

#[test]
fn test_wrong_num_columns() {
    let Signed { data, signature } = sign_with_random_keypair(true);
    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(encode(&signature)),
                    Value::String(encode(&data)),
                    Value::String(encode(&signature)),
                ],
            }],
        })],
    };
    list_intent_sets(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![Value::String(encode(&signature))],
            }],
        })],
    };
    list_intent_sets(queries).unwrap_err();
}

#[test]
fn test_wrong_num_rows() {
    let queries = QueryValues {
        queries: vec![Some(Rows { rows: vec![] })],
    };
    list_intent_sets(queries).unwrap_err();
}
