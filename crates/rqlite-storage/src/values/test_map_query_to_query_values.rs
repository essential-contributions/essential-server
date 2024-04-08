use super::*;
use serde_json::json;

#[test]
fn test_empty_result() {
    let json = json!({
    "results": [
        {
            "columns": [
                "bar",
                "foo"
            ],
            "types": [
                "",
                "blob"
            ]
        }
    ]})
    .as_object()
    .unwrap()
    .clone();

    let r = map_query_to_query_values(json).unwrap();
    let expected = QueryValues {
        queries: vec![None],
    };
    assert_eq!(r, expected);
}

#[test]
fn test_invalid() {
    let json = json!({
    "foo": [
    ]})
    .as_object()
    .unwrap()
    .clone();

    map_query_to_query_values(json).expect_err("Invalid format should result in error");
}

#[test]
fn test_no_query() {
    let json = json!({
    "results": [
    ]})
    .as_object()
    .unwrap()
    .clone();
    let r = map_query_to_query_values(json).unwrap();
    let expected = QueryValues { queries: vec![] };
    assert_eq!(r, expected);
}

#[test]
fn test_single_query() {
    let json = json!({
        "results": [
            {
                "columns": [
                    "bar",
                    "foo"
                ],
                "types": [
                    "text",
                    "blob"
                ],
                "values": [
                    [
                        "2",
                        "1"
                    ],
                    [
                        "3",
                        "2"
                    ]
                ]
            }
        ]
    })
    .as_object()
    .unwrap()
    .clone();

    let r = map_query_to_query_values(json).unwrap();
    let expected = QueryValues {
        queries: vec![Some(Rows {
            rows: vec![
                Columns {
                    columns: vec![
                        Value::String("2".to_string()),
                        Value::String("1".to_string()),
                    ],
                },
                Columns {
                    columns: vec![
                        Value::String("3".to_string()),
                        Value::String("2".to_string()),
                    ],
                },
            ],
        })],
    };
    assert_eq!(r, expected);
}

#[test]
fn test_multiple_queries() {
    let json = json!({
        "results": [
            {
                "columns": [
                    "bar",
                    "foo"
                ],
                "types": [
                    "text",
                    "blob"
                ],
                "values": [
                    [
                        "2",
                        "1"
                    ],
                    [
                        "3",
                        "2"
                    ]
                ]
            },
            {
                "columns": [
                    "id",
                    "foo",
                    "bar"
                ],
                "types": [
                    "integer",
                    "blob",
                    "text"
                ],
                "values": [
                    [
                        1,
                        "1",
                        "2"
                    ],
                    [
                        2,
                        "2",
                        "3"
                    ]
                ]
            }
        ]
    })
    .as_object()
    .unwrap()
    .clone();

    let r = map_query_to_query_values(json).unwrap();
    let expected = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![
                    Columns {
                        columns: vec![
                            Value::String("2".to_string()),
                            Value::String("1".to_string()),
                        ],
                    },
                    Columns {
                        columns: vec![
                            Value::String("3".to_string()),
                            Value::String("2".to_string()),
                        ],
                    },
                ],
            }),
            Some(Rows {
                rows: vec![
                    Columns {
                        columns: vec![
                            Value::Number(serde_json::Number::from(1)),
                            Value::String("1".to_string()),
                            Value::String("2".to_string()),
                        ],
                    },
                    Columns {
                        columns: vec![
                            Value::Number(serde_json::Number::from(2)),
                            Value::String("2".to_string()),
                            Value::String("3".to_string()),
                        ],
                    },
                ],
            }),
        ],
    };
    assert_eq!(r, expected);
}

#[test]
fn test_query_execute_query() {
    let json = json!({
        "results": [
            {
                "columns": [
                    "id",
                    "foo",
                    "bar"
                ],
                "types": [
                    "integer",
                    "blob",
                    "text"
                ],
                "values": [
                    [
                        1,
                        "1",
                        "2"
                    ],
                    [
                        2,
                        "2",
                        "3"
                    ]
                ]
            },
            {
                "last_insert_id": 3,
                "rows_affected": 1
            },
            {
                "columns": [
                    "id",
                    "foo",
                    "bar"
                ],
                "types": [
                    "integer",
                    "blob",
                    "text"
                ],
                "values": [
                    [
                        1,
                        "1",
                        "2"
                    ],
                    [
                        2,
                        "2",
                        "3"
                    ],
                    [
                        3,
                        "5",
                        "6"
                    ]
                ]
            }
        ]
    })
    .as_object()
    .unwrap()
    .clone();

    let r = map_query_to_query_values(json).unwrap();
    let expected = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![
                    Columns {
                        columns: vec![
                            Value::Number(serde_json::Number::from(1)),
                            Value::String("1".to_string()),
                            Value::String("2".to_string()),
                        ],
                    },
                    Columns {
                        columns: vec![
                            Value::Number(serde_json::Number::from(2)),
                            Value::String("2".to_string()),
                            Value::String("3".to_string()),
                        ],
                    },
                ],
            }),
            None,
            Some(Rows {
                rows: vec![
                    Columns {
                        columns: vec![
                            Value::Number(serde_json::Number::from(1)),
                            Value::String("1".to_string()),
                            Value::String("2".to_string()),
                        ],
                    },
                    Columns {
                        columns: vec![
                            Value::Number(serde_json::Number::from(2)),
                            Value::String("2".to_string()),
                            Value::String("3".to_string()),
                        ],
                    },
                    Columns {
                        columns: vec![
                            Value::Number(serde_json::Number::from(3)),
                            Value::String("5".to_string()),
                            Value::String("6".to_string()),
                        ],
                    },
                ],
            }),
        ],
    };
    assert_eq!(r, expected);
}
