use super::*;
use crate::encode;
use essential_types::contract::SignedContract;
use test_utils::{empty::Empty, predicate_with_salt, sign_contract_with_random_keypair};

#[test]
fn test_empty_query() {
    let queries = QueryValues {
        queries: vec![None, None],
    };

    assert_eq!(get_contract(queries).unwrap(), None);
}

#[test]
fn test_invalid_query() {
    let queries = QueryValues {
        queries: vec![None, None, None],
    };

    get_contract(queries).unwrap_err();
    let queries = QueryValues {
        queries: vec![None],
    };
    get_contract(queries).unwrap_err();

    let queries = QueryValues { queries: vec![] };

    get_contract(queries).unwrap_err();
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

    get_contract(queries).unwrap_err();
}

#[test]
fn test_signature_single_predicate() {
    let SignedContract {
        contract,
        signature,
    } = sign_contract_with_random_keypair(vec![Predicate::empty()].into());
    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&signature))],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&contract[0]))],
                }],
            }),
        ],
    };

    let r = get_contract(queries).unwrap().unwrap();
    let expected = SignedContract {
        contract: vec![Predicate::empty()].into(),
        signature,
    };
    assert_eq!(r, expected);
}

#[test]
fn test_invalid_data() {
    let invalid = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_string();
    let SignedContract {
        contract,
        signature,
    } = sign_contract_with_random_keypair(vec![Predicate::empty()].into());

    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(invalid.clone())],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&contract[0]))],
                }],
            }),
        ],
    };
    get_contract(queries).unwrap_err();

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
    get_contract(queries).unwrap_err();
}

#[test]
fn test_single_predicate_without_sig() {
    let SignedContract {
        contract,
        signature: _,
    } = sign_contract_with_random_keypair(vec![Predicate::empty()].into());
    let queries = QueryValues {
        queries: vec![
            None,
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&contract[0]))],
                }],
            }),
        ],
    };

    get_contract(queries).unwrap_err();
}

#[test]
fn test_signature_multiple_predicate() {
    let SignedContract {
        contract,
        signature,
    } = sign_contract_with_random_keypair(
        vec![predicate_with_salt(1), predicate_with_salt(2)].into(),
    );
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
                        columns: vec![Value::String(encode(&contract[0]))],
                    },
                    Columns {
                        columns: vec![Value::String(encode(&contract[1]))],
                    },
                ],
            }),
        ],
    };

    let r = get_contract(queries).unwrap().unwrap();
    let expected = SignedContract {
        contract: vec![predicate_with_salt(1), predicate_with_salt(2)].into(),
        signature,
    };
    assert_eq!(r, expected);
}

#[test]
fn test_invalid_signature_multiple_predicate() {
    let SignedContract {
        contract,
        signature: _,
    } = sign_contract_with_random_keypair(
        vec![predicate_with_salt(1), predicate_with_salt(2)].into(),
    );
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
                        Value::String(encode(&contract[0])),
                        Value::String(encode(&contract[1])),
                    ],
                }],
            }),
        ],
    };

    get_contract(queries).unwrap_err();
}

#[test]
fn test_signature_multiple_predicate_invalid() {
    let SignedContract {
        contract,
        signature,
    } = sign_contract_with_random_keypair(
        vec![predicate_with_salt(1), predicate_with_salt(2)].into(),
    );
    let queries = QueryValues {
        queries: vec![
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&signature))],
                }],
            }),
            Some(Rows {
                rows: vec![Columns {
                    columns: vec![Value::String(encode(&contract[0])), Value::Bool(true)],
                }],
            }),
        ],
    };

    get_contract(queries).unwrap_err();
}

#[test]
fn test_multi_column_sig() {
    let SignedContract {
        contract,
        signature,
    } = sign_contract_with_random_keypair(
        vec![predicate_with_salt(1), predicate_with_salt(2)].into(),
    );
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
                        Value::String(encode(&contract[0])),
                        Value::String(encode(&contract[1])),
                    ],
                }],
            }),
        ],
    };

    get_contract(queries).unwrap_err();
}

#[test]
fn test_multi_row_sig() {
    let SignedContract {
        contract,
        signature,
    } = sign_contract_with_random_keypair(
        vec![predicate_with_salt(1), predicate_with_salt(2)].into(),
    );
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
                        Value::String(encode(&contract[0])),
                        Value::String(encode(&contract[1])),
                    ],
                }],
            }),
        ],
    };

    get_contract(queries).unwrap_err();
}

#[test]
fn test_multi_row_predicate() {
    let SignedContract {
        contract,
        signature,
    } = sign_contract_with_random_keypair(
        vec![predicate_with_salt(1), predicate_with_salt(2)].into(),
    );
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
                            Value::String(encode(&contract[0])),
                            Value::String(encode(&contract[1])),
                        ],
                    },
                    Columns {
                        columns: vec![
                            Value::String(encode(&contract[0])),
                            Value::String(encode(&contract[1])),
                        ],
                    },
                ],
            }),
        ],
    };

    get_contract(queries).unwrap_err();
}
