use std::{ops::Range, time::Duration};

use rusqlite::{params, Connection};

use common::*;

mod common;

fn insert_state(conn: &Connection, content_hash: usize, range: Range<usize>) {
    let sql = include_sql!("update", "update_state");
    insert(conn, sql, content_hash, range);
}

fn insert_eoa_state(conn: &Connection, content_hash: usize, range: Range<usize>) {
    let sql = include_sql!("update", "update_eoa_state");
    insert(conn, sql, content_hash, range);
}

fn insert(conn: &Connection, sql: &str, content_hash: usize, range: Range<usize>) {
    for i in range {
        conn.execute(
            sql,
            params![format!("key{}", i), i, format!("hash{}", content_hash),],
        )
        .unwrap();
    }
}

fn delete_state(conn: &Connection, content_hash: usize, range: Range<usize>) {
    let sql = include_sql!("update", "delete_state");
    delete(conn, sql, content_hash, range)
}

fn delete_eoa_state(conn: &Connection, content_hash: usize, range: Range<usize>) {
    let sql = include_sql!("update", "delete_eoa_state");
    delete(conn, sql, content_hash, range)
}

fn delete(conn: &Connection, sql: &str, content_hash: usize, range: Range<usize>) {
    for i in range {
        conn.execute(
            sql,
            params![format!("hash{}", content_hash), format!("key{}", i)],
        )
        .unwrap();
    }
}

fn get_state(conn: &Connection, content_hash: usize, range: Range<usize>) -> Vec<usize> {
    let sql = include_sql!("query", "get_state");
    get(conn, sql, content_hash, range)
}

fn get_eoa_state(conn: &Connection, content_hash: usize, range: Range<usize>) -> Vec<usize> {
    let sql = include_sql!("query", "get_eoa_state");
    get(conn, sql, content_hash, range)
}

fn get(conn: &Connection, sql: &str, content_hash: usize, range: Range<usize>) -> Vec<usize> {
    let mut out = Vec::new();
    for i in range {
        let r = query(
            conn,
            sql,
            [format!("hash{}", content_hash), format!("key{}", i)],
            |row| row.get::<_, usize>(0).unwrap(),
        );
        out.extend(r);
    }
    out
}

#[test]
fn test_update_state() {
    let conn = Connection::open_in_memory().unwrap();
    create_tables(&conn);

    insert_intent_set(&conn, 1, Duration::from_secs(1), 0..2);

    insert_state(&conn, 1, 20..300);
    insert_state(&conn, 1, 20..300);

    let result = get_state(&conn, 1, 20..21);
    assert_eq!(result, vec![20]);
    let result = get_state(&conn, 1, 19..21);
    assert_eq!(result, vec![20]);
    let result = get_state(&conn, 1, 299..350);
    assert_eq!(result, vec![299]);

    delete_state(&conn, 1, 150..300);
    let result = get_state(&conn, 1, 150..300);
    assert_eq!(result, vec![]);
    let result = get_state(&conn, 1, 149..150);
    assert_eq!(result, vec![149]);
}

#[test]
fn test_update_eoa_state() {
    let conn = Connection::open_in_memory().unwrap();
    create_tables(&conn);

    conn.execute(include_sql!("insert", "eoa"), params![format!("hash{}", 1)])
        .unwrap();

    insert_eoa_state(&conn, 1, 20..300);
    insert_eoa_state(&conn, 1, 20..300);

    let result = get_eoa_state(&conn, 1, 20..21);
    assert_eq!(result, vec![20]);
    let result = get_eoa_state(&conn, 1, 19..21);
    assert_eq!(result, vec![20]);
    let result = get_eoa_state(&conn, 1, 299..350);
    assert_eq!(result, vec![299]);

    delete_eoa_state(&conn, 1, 150..300);
    let result = get_eoa_state(&conn, 1, 150..300);
    assert_eq!(result, vec![]);
    let result = get_eoa_state(&conn, 1, 149..150);
    assert_eq!(result, vec![149]);
}
