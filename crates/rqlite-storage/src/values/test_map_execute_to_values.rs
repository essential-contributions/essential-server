use crate::encode;

use super::*;
use serde_json::json;

#[test]
fn test_valid_result() {
    let json = json!({
            "results": [
                {
                    "columns": [
                        "foo",
                    ],
                    "types": [
                        "blob",
                    ],
                    "values": [
                        [
                            encode(&vec![1, 2]),
                        ]
                    ]
                },
                {
                    "last_insert_id": 3,
                    "rows_affected": 1
                }
            ]
    })
    .as_object()
    .unwrap()
    .clone();

    let r = map_execute_to_values(json).unwrap();
    assert_eq!(r, vec![1, 2]);
}

#[test]
fn test_empty_result() {
    let json = json!({
            "results": [
                {
                    "columns": [
                        "foo",
                    ],
                    "types": [
                        "integer",
                    ],
                },
                {
                    "last_insert_id": 3,
                    "rows_affected": 1
                }
            ]
    })
    .as_object()
    .unwrap()
    .clone();

    let r = map_execute_to_values(json).unwrap();
    assert!(r.is_empty());
}

#[test]
fn test_missing_results() {
    let json = json!({
            "foo": [
                {
                    "columns": [
                        "foo",
                    ],
                    "types": [
                        "integer",
                    ],
                },
                {
                    "last_insert_id": 3,
                    "rows_affected": 1
                }
            ]
    })
    .as_object()
    .unwrap()
    .clone();

    map_execute_to_values(json).unwrap_err();
}

#[test]
fn test_missing_execute() {
    let json = json!({
            "results": [
                {
                    "columns": [
                        "foo",
                    ],
                    "types": [
                        "integer",
                    ],
                }
            ]
    })
    .as_object()
    .unwrap()
    .clone();

    map_execute_to_values(json).unwrap_err();
}

#[test]
fn test_invalid_result() {
    let json = json!({
            "results": [
                {
                    "columns": [
                        "foo",
                    ],
                    "types": [
                        "integer",
                    ],
                    "values": [
                        [
                            1.0,
                        ]
                    ]
                },
                {
                    "last_insert_id": 3,
                    "rows_affected": 1
                }
            ]
    })
    .as_object()
    .unwrap()
    .clone();

    map_execute_to_values(json).unwrap_err();

    let json = json!({
            "results": [
                {
                    "columns": [
                        "foo",
                    ],
                    "types": [
                        "integer",
                    ],
                    "values": [
                        [
                            "foo",
                        ]
                    ]
                },
                {
                    "last_insert_id": 3,
                    "rows_affected": 1
                }
            ]
    })
    .as_object()
    .unwrap()
    .clone();

    map_execute_to_values(json).unwrap_err();

    let json = json!({
            "results": [
                {
                    "columns": [
                        "foo",
                    ],
                    "types": [
                        "integer",
                    ],
                    "values": [
                        [
                            1,
                            1,
                        ]
                    ]
                },
                {
                    "last_insert_id": 3,
                    "rows_affected": 1
                }
            ]
    })
    .as_object()
    .unwrap()
    .clone();

    map_execute_to_values(json).unwrap_err();

    let json = json!({
            "results": [
                {
                    "columns": [
                        "foo",
                    ],
                    "types": [
                        "integer",
                    ],
                    "values": [
                        []
                    ]
                },
                {
                    "last_insert_id": 3,
                    "rows_affected": 1
                }
            ]
    })
    .as_object()
    .unwrap()
    .clone();

    map_execute_to_values(json).unwrap_err();
}
