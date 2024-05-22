use super::*;
use crate::encode;
use test_utils::empty::Empty;

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
    let solution = Solution::empty();
    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![Value::String(encode(&solution))],
            }],
        })],
    };

    let r = list_solutions::<Solution>(queries).unwrap();
    let expected = vec![solution.clone()];
    assert_eq!(r, expected);

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

    let r = list_solutions::<Solution>(queries).unwrap();
    let expected = vec![solution.clone(), solution];
    assert_eq!(r, expected);
}

#[test]
fn test_invalid_data() {
    let invalid = "xxxxxxx".to_string();
    let solution = Solution::empty();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![
                    Value::String(invalid.clone()),
                    Value::String(encode(&solution)),
                ],
            }],
        })],
    };
    list_solutions::<Solution>(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![Value::String(invalid.clone())],
            }],
        })],
    };
    list_solutions::<Solution>(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![Value::Bool(true), Value::String(encode(&solution))],
            }],
        })],
    };
    list_solutions::<Solution>(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns {
                columns: vec![Value::String(encode(&solution)), Value::Bool(true)],
            }],
        })],
    };
    list_solutions::<Solution>(queries).unwrap_err();
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
    list_solutions::<Solution>(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![Columns { columns: vec![] }],
        })],
    };
    list_solutions::<Solution>(queries).unwrap_err();
}

#[test]
fn test_wrong_num_rows() {
    let queries = QueryValues {
        queries: vec![Some(Rows { rows: vec![] })],
    };
    list_solutions::<Solution>(queries).unwrap_err();
}
