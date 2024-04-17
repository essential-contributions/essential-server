use super::*;
use crate::encode;
use serde_json::Number;
use test_utils::intent_with_decision_variables;

#[test]
fn test_empty_query() {
    let queries = QueryValues {
        queries: vec![None],
    };

    assert!(list_intent_sets(queries).unwrap().is_empty());
}

#[test]
fn test_invalid_query() {
    let queries = QueryValues { queries: vec![] };

    list_intent_sets(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![None, None],
    };

    list_intent_sets(queries).unwrap_err();
}

#[test]
fn test_valid_queries() {
    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::Number(Number::from(1)),
                    Value::String(encode(&intent_with_decision_variables(1))),
                ],
            }],
        })],
    };

    let r = list_intent_sets(queries).unwrap();
    let expected = vec![vec![intent_with_decision_variables(1)]];
    assert_eq!(r, expected);

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![
                Columns {
                    columns: vec![
                        Value::Number(Number::from(0)),
                        Value::String(encode(&intent_with_decision_variables(1))),
                    ],
                },
                Columns {
                    columns: vec![
                        Value::Number(Number::from(1)),
                        Value::String(encode(&intent_with_decision_variables(1))),
                    ],
                },
                Columns {
                    columns: vec![
                        Value::Number(Number::from(1)),
                        Value::String(encode(&intent_with_decision_variables(2))),
                    ],
                },
            ],
        })],
    };

    let r = list_intent_sets(queries).unwrap();
    let expected = vec![
        vec![intent_with_decision_variables(1)],
        vec![
            intent_with_decision_variables(1),
            intent_with_decision_variables(2),
        ],
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
    list_intent_sets(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::Bool(true),
                    Value::String(encode(&intent_with_decision_variables(1))),
                ],
            }],
        })],
    };
    list_intent_sets(queries).unwrap_err();

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
    list_intent_sets(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::Number(Number::from_f64(1.0).unwrap()),
                    Value::String(encode(&intent_with_decision_variables(1))),
                ],
            }],
        })],
    };
    list_intent_sets(queries).unwrap_err();
}

#[test]
fn test_wrong_num_columns() {
    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::Number(Number::from(1)),
                    Value::String(encode(&intent_with_decision_variables(1))),
                    Value::Number(Number::from(1)),
                ],
            }],
        })],
    };
    list_intent_sets(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![Value::Number(Number::from(1))],
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
