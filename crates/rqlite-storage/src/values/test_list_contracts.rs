use super::*;
use crate::encode;
use serde_json::Number;
use test_utils::predicate_with_salt;

#[test]
fn test_empty_query() {
    let queries = QueryValues {
        queries: vec![None, None],
    };

    assert!(list_contracts(queries).unwrap().is_empty());
}

#[test]
fn test_invalid_query() {
    let queries = QueryValues { queries: vec![] };

    list_contracts(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![None, None, None],
    };

    list_contracts(queries).unwrap_err();
}

#[test]
fn test_valid_queries() {
    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![
                        Value::Number(Number::from(1)),
                        Value::String(encode(&[0; 32])),
                    ],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![
                        Value::Number(Number::from(1)),
                        Value::String(encode(&predicate_with_salt(1))),
                    ],
                }],
            }),
        ],
    };

    let r = list_contracts(queries).unwrap();
    let expected = vec![vec![predicate_with_salt(1)].into()];
    assert_eq!(r, expected);

    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![
                    Columns {
                        columns: vec![
                            Value::Number(Number::from(0)),
                            Value::String(encode(&[0; 32])),
                        ],
                    },
                    Columns {
                        columns: vec![
                            Value::Number(Number::from(1)),
                            Value::String(encode(&[0; 32])),
                        ],
                    },
                ],
            }),
            Some(Rows {
                rows: vec![
                    Columns {
                        columns: vec![
                            Value::Number(Number::from(0)),
                            Value::String(encode(&predicate_with_salt(1))),
                        ],
                    },
                    Columns {
                        columns: vec![
                            Value::Number(Number::from(1)),
                            Value::String(encode(&predicate_with_salt(1))),
                        ],
                    },
                    Columns {
                        columns: vec![
                            Value::Number(Number::from(1)),
                            Value::String(encode(&predicate_with_salt(2))),
                        ],
                    },
                ],
            }),
        ],
    };

    let r = list_contracts(queries).unwrap();
    let expected = vec![
        vec![predicate_with_salt(1)].into(),
        vec![predicate_with_salt(1), predicate_with_salt(2)].into(),
    ];
    assert_eq!(r, expected);
}

#[test]
fn test_invalid_data() {
    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![Value::Number(Number::from(1)), Value::Bool(true)],
            }],
        })],
    };
    list_contracts(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::Bool(true),
                    Value::String(encode(&predicate_with_salt(1))),
                ],
            }],
        })],
    };
    list_contracts(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::Number(Number::from(1)),
                    Value::String("xxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_string()),
                ],
            }],
        })],
    };
    list_contracts(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::Number(Number::from_f64(1.0).unwrap()),
                    Value::String(encode(&predicate_with_salt(1))),
                ],
            }],
        })],
    };
    list_contracts(queries).unwrap_err();
}

#[test]
fn test_wrong_num_columns() {
    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::Number(Number::from(1)),
                    Value::String(encode(&predicate_with_salt(1))),
                    Value::Number(Number::from(1)),
                ],
            }],
        })],
    };
    list_contracts(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![Value::Number(Number::from(1))],
            }],
        })],
    };
    list_contracts(queries).unwrap_err();
}

#[test]
fn test_wrong_num_rows() {
    let queries = QueryValues {
        queries: vec![Some(Rows { rows: vec![] })],
    };
    list_contracts(queries).unwrap_err();
}
