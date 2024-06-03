use super::*;
use crate::encode;
use test_utils::{empty::Empty, intent_with_salt, sign_intent_set_with_random_keypair};

#[test]
fn test_empty_query() {
    let queries = QueryValues {
        queries: vec![None, None],
    };

    assert_eq!(get_intent_set(queries).unwrap(), None);
}

#[test]
fn test_invalid_query() {
    let queries = QueryValues {
        queries: vec![None, None, None],
    };

    get_intent_set(queries).unwrap_err();
    let queries = QueryValues {
        queries: vec![None],
    };
    get_intent_set(queries).unwrap_err();

    let queries = QueryValues { queries: vec![] };

    get_intent_set(queries).unwrap_err();
}

#[test]
fn test_signature_only() {
    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String("signature".to_string())],
                }],
            }),
            None,
        ],
    };

    get_intent_set(queries).unwrap_err();
}

#[test]
fn test_signature_single_intent() {
    let intent::SignedSet { set, signature } =
        sign_intent_set_with_random_keypair(vec![Intent::empty()]);
    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&signature))],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&set[0]))],
                }],
            }),
        ],
    };

    let r = get_intent_set(queries).unwrap().unwrap();
    let expected = intent::SignedSet {
        set: vec![Intent::empty()],
        signature,
    };
    assert_eq!(r, expected);
}

#[test]
fn test_invalid_data() {
    let invalid = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_string();
    let intent::SignedSet { set, signature } =
        sign_intent_set_with_random_keypair(vec![Intent::empty()]);

    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(invalid.clone())],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&set[0]))],
                }],
            }),
        ],
    };
    get_intent_set(queries).unwrap_err();

    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&signature))],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(invalid.clone())],
                }],
            }),
        ],
    };
    get_intent_set(queries).unwrap_err();
}

#[test]
fn test_single_intent_without_sig() {
    let intent::SignedSet { set, signature: _ } =
        sign_intent_set_with_random_keypair(vec![Intent::empty()]);
    let queries = QueryValues {
        queries: vec![
            None,
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&set[0]))],
                }],
            }),
        ],
    };

    get_intent_set(queries).unwrap_err();
}

#[test]
fn test_signature_multiple_intent() {
    let intent::SignedSet { set, signature } =
        sign_intent_set_with_random_keypair(vec![intent_with_salt(1), intent_with_salt(2)]);
    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&signature))],
                }],
            }),
            Some(Rows {
                rows: vec![
                    Columns {
                        columns: vec![Value::String(encode(&set[0]))],
                    },
                    Columns {
                        columns: vec![Value::String(encode(&set[1]))],
                    },
                ],
            }),
        ],
    };

    let r = get_intent_set(queries).unwrap().unwrap();
    let expected = intent::SignedSet {
        set: vec![intent_with_salt(1), intent_with_salt(2)],
        signature,
    };
    assert_eq!(r, expected);
}

#[test]
fn test_invalid_signature_multiple_intent() {
    let intent::SignedSet { set, signature: _ } =
        sign_intent_set_with_random_keypair(vec![intent_with_salt(1), intent_with_salt(2)]);
    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::Bool(true)],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![
                        Value::String(encode(&set[0])),
                        Value::String(encode(&set[1])),
                    ],
                }],
            }),
        ],
    };

    get_intent_set(queries).unwrap_err();
}

#[test]
fn test_signature_multiple_intent_invalid() {
    let intent::SignedSet { set, signature } =
        sign_intent_set_with_random_keypair(vec![intent_with_salt(1), intent_with_salt(2)]);
    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&signature))],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&set[0])), Value::Bool(true)],
                }],
            }),
        ],
    };

    get_intent_set(queries).unwrap_err();
}

#[test]
fn test_multi_column_sig() {
    let intent::SignedSet { set, signature } =
        sign_intent_set_with_random_keypair(vec![intent_with_salt(1), intent_with_salt(2)]);
    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![
                        Value::String(encode(&signature)),
                        Value::String(encode(&signature)),
                    ],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![
                        Value::String(encode(&set[0])),
                        Value::String(encode(&set[1])),
                    ],
                }],
            }),
        ],
    };

    get_intent_set(queries).unwrap_err();
}

#[test]
fn test_multi_row_sig() {
    let intent::SignedSet { set, signature } =
        sign_intent_set_with_random_keypair(vec![intent_with_salt(1), intent_with_salt(2)]);
    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![
                    Columns {
                        columns: vec![Value::String(encode(&signature))],
                    },
                    Columns {
                        columns: vec![Value::String(encode(&signature))],
                    },
                ],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![
                        Value::String(encode(&set[0])),
                        Value::String(encode(&set[1])),
                    ],
                }],
            }),
        ],
    };

    get_intent_set(queries).unwrap_err();
}

#[test]
fn test_multi_row_intent() {
    let intent::SignedSet { set, signature } =
        sign_intent_set_with_random_keypair(vec![intent_with_salt(1), intent_with_salt(2)]);
    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&signature))],
                }],
            }),
            Some(Rows {
                rows: vec![
                    Columns {
                        columns: vec![
                            Value::String(encode(&set[0])),
                            Value::String(encode(&set[1])),
                        ],
                    },
                    Columns {
                        columns: vec![
                            Value::String(encode(&set[0])),
                            Value::String(encode(&set[1])),
                        ],
                    },
                ],
            }),
        ],
    };

    get_intent_set(queries).unwrap_err();
}
