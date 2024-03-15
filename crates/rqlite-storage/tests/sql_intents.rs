use std::time::Duration;

use rusqlite::{named_params, Connection};

use common::*;

mod common;

#[test]
fn test_insert_intent_set_double_insert() {
    let conn = Connection::open_in_memory().unwrap();
    create_tables(&conn);

    // Double insert is a noop
    insert_intent_set(&conn, 1, Duration::from_secs(1), 0..2);
    insert_intent_set(&conn, 1, Duration::from_secs(1), 0..2);

    let result = query(&conn, "select * from intent_sets", [], |row| {
        (
            row.get::<_, usize>(0).unwrap(),
            row.get::<_, String>(1).unwrap(),
            row.get::<_, String>(2).unwrap(),
            row.get::<_, usize>(3).unwrap(),
            row.get::<_, usize>(4).unwrap(),
        )
    });
    assert_eq!(
        result,
        vec![(1, "hash1".to_string(), "signature1".to_string(), 1, 0,)]
    );

    let result = query(&conn, "select * from intents", [], |row| {
        (
            row.get::<_, usize>(0).unwrap(),
            row.get::<_, String>(1).unwrap(),
            row.get::<_, String>(2).unwrap(),
        )
    });
    assert_eq!(
        result,
        vec![
            (1, "intent0".to_string(), "intent_hash0".to_string(),),
            (2, "intent1".to_string(), "intent_hash1".to_string(),),
        ]
    );

    let result = query(&conn, "select * from intent_set_pairing", [], |row| {
        (
            row.get::<_, usize>(0).unwrap(),
            row.get::<_, usize>(1).unwrap(),
            row.get::<_, usize>(2).unwrap(),
        )
    });
    assert_eq!(result, vec![(1, 1, 1), (2, 1, 2),]);
}

#[test]
fn test_intent_gets() {
    let conn = Connection::open_in_memory().unwrap();
    create_tables(&conn);

    insert_intent_set(&conn, 1, Duration::from_secs(1), 0..2);
    insert_intent_set(&conn, 2, Duration::from_secs(2), 1..3);

    let result = query(
        &conn,
        include_sql!("query", "get_intent_set_signature"),
        ["hash1"],
        |row| row.get::<_, String>(0).unwrap(),
    );
    assert_eq!(result, vec!["signature1".to_string()]);

    let result = query(
        &conn,
        include_sql!("query", "get_intent_set_signature"),
        ["hash2"],
        |row| row.get::<_, String>(0).unwrap(),
    );
    assert_eq!(result, vec!["signature2".to_string()]);

    let result = query(
        &conn,
        include_sql!("query", "get_intent"),
        ["intent_hash1"],
        |row| row.get::<_, String>(0).unwrap(),
    );
    assert_eq!(result, vec!["intent1".to_string()]);

    let result = query(
        &conn,
        include_sql!("query", "get_intent"),
        ["intent_hash2"],
        |row| row.get::<_, String>(0).unwrap(),
    );
    assert_eq!(result, vec!["intent2".to_string()]);

    let result = query(
        &conn,
        include_sql!("query", "get_intent_set"),
        ["hash1"],
        |row| row.get::<_, String>(0).unwrap(),
    );
    assert_eq!(result, vec!["intent0".to_string(), "intent1".to_string(),]);

    let result = query(
        &conn,
        include_sql!("query", "get_intent_set"),
        ["hash2"],
        |row| row.get::<_, String>(0).unwrap(),
    );
    assert_eq!(result, vec!["intent1".to_string(), "intent2".to_string(),]);
}

#[test]
fn test_list_intent_sets() {
    let conn = Connection::open_in_memory().unwrap();
    create_tables(&conn);

    insert_intent_set(&conn, 1, Duration::from_secs(1), 0..2);
    insert_intent_set(&conn, 2, Duration::from_secs(2), 1..3);

    let result = query(
        &conn,
        include_sql!("query", "list_intent_sets"),
        named_params! {
            ":page_size": 1,
            ":page_number": 0,
        },
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
            )
        },
    );

    assert_eq!(
        result,
        vec![(1, "intent0".to_string()), (1, "intent1".to_string()),]
    );

    let result = query(
        &conn,
        include_sql!("query", "list_intent_sets"),
        named_params! {
            ":page_size": 1,
            ":page_number": 1,
        },
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
            )
        },
    );

    assert_eq!(
        result,
        vec![(2, "intent1".to_string()), (2, "intent2".to_string()),]
    );

    let result = query(
        &conn,
        include_sql!("query", "list_intent_sets"),
        named_params! {
            ":page_size": 2,
            ":page_number": 0,
        },
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
            )
        },
    );

    assert_eq!(
        result,
        vec![
            (1, "intent0".to_string()),
            (1, "intent1".to_string()),
            (2, "intent1".to_string()),
            (2, "intent2".to_string()),
        ]
    );
}

#[test]
fn test_list_intent_sets_by_time() {
    let conn = Connection::open_in_memory().unwrap();
    create_tables(&conn);

    insert_intent_set(&conn, 1, Duration::new(22, 33), 0..2);
    insert_intent_set(&conn, 2, Duration::new(44, 12), 1..3);

    let result = query(
        &conn,
        include_sql!("query", "list_intent_sets_by_time"),
        named_params! {
            ":page_size": 1,
            ":page_number": 0,
            ":start_seconds": 0,
            ":start_nanos": 0,
            ":end_seconds": 100,
            ":end_nanos": 100,
        },
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
            )
        },
    );

    assert_eq!(
        result,
        vec![(1, "intent0".to_string()), (1, "intent1".to_string()),]
    );

    let result = query(
        &conn,
        include_sql!("query", "list_intent_sets_by_time"),
        named_params! {
            ":page_size": 1,
            ":page_number": 0,
            ":start_seconds": 44,
            ":start_nanos": 0,
            ":end_seconds": 100,
            ":end_nanos": 100,
        },
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
            )
        },
    );
    assert_eq!(
        result,
        vec![(2, "intent1".to_string()), (2, "intent2".to_string()),]
    );

    let result = query(
        &conn,
        include_sql!("query", "list_intent_sets_by_time"),
        named_params! {
            ":page_size": 1,
            ":page_number": 0,
            ":start_seconds": 11,
            ":start_nanos": 44,
            ":end_seconds": 100,
            ":end_nanos": 100,
        },
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
            )
        },
    );

    assert_eq!(
        result,
        vec![(1, "intent0".to_string()), (1, "intent1".to_string()),]
    );

    let result = query(
        &conn,
        include_sql!("query", "list_intent_sets_by_time"),
        named_params! {
            ":page_size": 2,
            ":page_number": 0,
            ":start_seconds": 22,
            ":start_nanos": 0,
            ":end_seconds": 100,
            ":end_nanos": 100,
        },
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
            )
        },
    );

    assert_eq!(
        result,
        vec![
            (1, "intent0".to_string()),
            (1, "intent1".to_string()),
            (2, "intent1".to_string()),
            (2, "intent2".to_string()),
        ]
    );
}
