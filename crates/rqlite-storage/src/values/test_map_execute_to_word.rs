use serde_json::json;

use super::*;

#[test]
fn test_valid_result() {
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

    let r = map_execute_to_word(json).unwrap().unwrap();
    assert_eq!(r, 1);
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

    let r = map_execute_to_word(json).unwrap();
    assert_eq!(r, None);
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

    map_execute_to_word(json).unwrap_err();
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

    map_execute_to_word(json).unwrap_err();
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

    map_execute_to_word(json).unwrap_err();

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

    map_execute_to_word(json).unwrap_err();

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

    map_execute_to_word(json).unwrap_err();

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

    map_execute_to_word(json).unwrap_err();
}
