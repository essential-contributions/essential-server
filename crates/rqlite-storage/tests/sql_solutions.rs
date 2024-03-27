use std::time::Duration;

use rusqlite::{named_params, params, Connection};

use common::*;

mod common;

#[test]
fn test_insert_solutions() {
    let conn = Connection::open_in_memory().unwrap();
    create_tables(&conn);

    // Double insert is a noop
    conn.execute(
        include_sql!("insert", "solutions_pool"),
        ["hash1", "solution1", "signature1"],
    )
    .unwrap();

    conn.execute(
        include_sql!("insert", "solutions_pool"),
        ["hash1", "solution1", "signature1"],
    )
    .unwrap();

    let result = query(&conn, "select * from solutions_pool", [], |row| {
        (
            row.get::<_, usize>(0).unwrap(),
            row.get::<_, String>(1).unwrap(),
            row.get::<_, String>(2).unwrap(),
            row.get::<_, String>(3).unwrap(),
        )
    });
    assert_eq!(
        result,
        vec![(
            1,
            "hash1".to_string(),
            "solution1".to_string(),
            "signature1".to_string()
        )]
    );

    // Can insert a second solution
    conn.execute(
        include_sql!("insert", "solutions_pool"),
        ["hash2", "solution2", "signature2"],
    )
    .unwrap();

    let result = query(&conn, "select * from solutions_pool", [], |row| {
        (
            row.get::<_, usize>(0).unwrap(),
            row.get::<_, String>(1).unwrap(),
            row.get::<_, String>(2).unwrap(),
            row.get::<_, String>(3).unwrap(),
        )
    });
    assert_eq!(
        result,
        vec![
            (
                1,
                "hash1".to_string(),
                "solution1".to_string(),
                "signature1".to_string()
            ),
            (
                2,
                "hash2".to_string(),
                "solution2".to_string(),
                "signature2".to_string()
            ),
        ]
    );

    // list solutions pool
    let result = query(
        &conn,
        include_sql!("query", "list_solutions_pool"),
        [],
        |row| {
            (
                row.get::<_, String>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
            )
        },
    );
    assert_eq!(
        result,
        vec![
            ("signature1".to_string(), "solution1".to_string(),),
            ("signature2".to_string(), "solution2".to_string(),),
        ]
    );

    // Move solutions to solved
    conn.execute(
        include_sql!("insert", "batch"),
        params!["batch_hash1", 0, 0],
    )
    .unwrap();
    conn.execute(include_sql!("insert", "copy_to_solved"), ["hash1"])
        .unwrap();
    conn.execute(include_sql!("insert", "copy_to_solved"), ["hash2"])
        .unwrap();
    conn.execute(
        include_sql!("update", "delete_from_solutions_pool"),
        ["hash1"],
    )
    .unwrap();
    conn.execute(
        include_sql!("update", "delete_from_solutions_pool"),
        ["hash2"],
    )
    .unwrap();

    // pool is empty
    let result = query(
        &conn,
        include_sql!("query", "list_solutions_pool"),
        [],
        |row| {
            (
                row.get::<_, String>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
            )
        },
    );
    assert_eq!(result, vec![]);

    // list winning batches
    let result = query(
        &conn,
        include_sql!("query", "list_winning_batches"),
        named_params! {
            ":page_size": 10,
            ":page_number": 0,
        },
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
                row.get::<_, String>(2).unwrap(),
                row.get::<_, usize>(3).unwrap(),
                row.get::<_, usize>(4).unwrap(),
            )
        },
    );

    assert_eq!(
        result,
        vec![
            (1, "solution1".to_string(), "signature1".to_string(), 0, 0),
            (1, "solution2".to_string(), "signature2".to_string(), 0, 0),
        ]
    );
}

#[test]
fn test_insert_partial_solutions() {
    let conn = Connection::open_in_memory().unwrap();
    create_tables(&conn);

    // Double insert is a noop
    conn.execute(
        include_sql!("insert", "partial_solutions"),
        ["hash1", "solution1", "signature1"],
    )
    .unwrap();

    conn.execute(
        include_sql!("insert", "partial_solutions"),
        ["hash1", "solution1", "signature1"],
    )
    .unwrap();

    let result = query(&conn, "select * from partial_solutions", [], |row| {
        (
            row.get::<_, usize>(0).unwrap(),
            row.get::<_, String>(1).unwrap(),
            row.get::<_, String>(2).unwrap(),
            row.get::<_, String>(3).unwrap(),
            row.get::<_, bool>(4).unwrap(),
        )
    });
    assert_eq!(
        result,
        vec![(
            1,
            "hash1".to_string(),
            "solution1".to_string(),
            "signature1".to_string(),
            false
        )]
    );

    // Can insert a second solution
    conn.execute(
        include_sql!("insert", "partial_solutions"),
        ["hash2", "solution2", "signature2"],
    )
    .unwrap();

    let result = query(&conn, "select * from partial_solutions", [], |row| {
        (
            row.get::<_, usize>(0).unwrap(),
            row.get::<_, String>(1).unwrap(),
            row.get::<_, String>(2).unwrap(),
            row.get::<_, String>(3).unwrap(),
            row.get::<_, bool>(4).unwrap(),
        )
    });
    assert_eq!(
        result,
        vec![
            (
                1,
                "hash1".to_string(),
                "solution1".to_string(),
                "signature1".to_string(),
                false
            ),
            (
                2,
                "hash2".to_string(),
                "solution2".to_string(),
                "signature2".to_string(),
                false
            ),
        ]
    );

    // list solutions pool
    let result = query(
        &conn,
        include_sql!("query", "list_partial_solutions"),
        [],
        |row| {
            (
                row.get::<_, String>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
            )
        },
    );
    assert_eq!(
        result,
        vec![
            ("signature1".to_string(), "solution1".to_string(),),
            ("signature2".to_string(), "solution2".to_string(),),
        ]
    );

    let result = query(
        &conn,
        include_sql!("query", "is_partial_solution_solved"),
        ["hash1"],
        |row| row.get::<_, bool>(0).unwrap(),
    );
    assert_eq!(result, vec![false]);

    let result = query(
        &conn,
        include_sql!("query", "is_partial_solution_solved"),
        ["hash2"],
        |row| row.get::<_, bool>(0).unwrap(),
    );
    assert_eq!(result, vec![false]);

    // Move solutions to solved
    conn.execute(
        include_sql!("update", "set_partial_solution_to_solved"),
        ["hash1"],
    )
    .unwrap();
    conn.execute(
        include_sql!("update", "set_partial_solution_to_solved"),
        ["hash2"],
    )
    .unwrap();

    let result = query(
        &conn,
        include_sql!("query", "is_partial_solution_solved"),
        ["hash1"],
        |row| row.get::<_, bool>(0).unwrap(),
    );
    assert_eq!(result, vec![true]);

    let result = query(
        &conn,
        include_sql!("query", "is_partial_solution_solved"),
        ["hash2"],
        |row| row.get::<_, bool>(0).unwrap(),
    );
    assert_eq!(result, vec![true]);

    // pool is empty
    let result = query(
        &conn,
        include_sql!("query", "list_partial_solutions"),
        [],
        |row| {
            (
                row.get::<_, String>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
            )
        },
    );
    assert_eq!(result, vec![]);

    // Get partial solutions
    let result = query(
        &conn,
        include_sql!("query", "get_partial_solution"),
        ["hash1"],
        |row| {
            (
                row.get::<_, String>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
            )
        },
    );

    assert_eq!(
        result,
        vec![("solution1".to_string(), "signature1".to_string()),]
    );

    let result = query(
        &conn,
        include_sql!("query", "get_partial_solution"),
        ["hash2"],
        |row| {
            (
                row.get::<_, String>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
            )
        },
    );

    assert_eq!(
        result,
        vec![("solution2".to_string(), "signature2".to_string()),]
    );
}

#[test]
fn test_batch_paging() {
    let conn = Connection::open_in_memory().unwrap();
    create_tables(&conn);

    for n in 0..1000 {
        let start = n * 2;
        let end = start + 2;
        let hashes = (start..end)
            .map(|i| format!("hash{}", i))
            .collect::<Vec<_>>();

        for (hash, i) in hashes.iter().zip(start..end) {
            conn.execute(
                include_sql!("insert", "solutions_pool"),
                [
                    hash.to_string(),
                    format!("solution{}", i),
                    format!("signature{}", i),
                ],
            )
            .unwrap();
        }

        move_solutions_to_solved(
            &conn,
            n,
            &hashes,
            Duration::new((100 * n) as u64, (100 * n) as u32),
        );
    }

    let result = query(
        &conn,
        include_sql!("query", "list_winning_batches"),
        named_params! {
            ":page_size": 2,
            ":page_number": 0,
        },
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
                row.get::<_, String>(2).unwrap(),
                row.get::<_, usize>(3).unwrap(),
                row.get::<_, usize>(4).unwrap(),
            )
        },
    );
    assert_eq!(
        result,
        vec![
            (1, "solution0".to_string(), "signature0".to_string(), 0, 0),
            (1, "solution1".to_string(), "signature1".to_string(), 0, 0),
            (
                2,
                "solution2".to_string(),
                "signature2".to_string(),
                100,
                100
            ),
            (
                2,
                "solution3".to_string(),
                "signature3".to_string(),
                100,
                100
            ),
        ]
    );

    let result = query(
        &conn,
        include_sql!("query", "list_winning_batches"),
        named_params! {
            ":page_size": 2,
            ":page_number": 1,
        },
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
                row.get::<_, String>(2).unwrap(),
                row.get::<_, usize>(3).unwrap(),
                row.get::<_, usize>(4).unwrap(),
            )
        },
    );
    assert_eq!(
        result,
        vec![
            (
                3,
                "solution4".to_string(),
                "signature4".to_string(),
                200,
                200
            ),
            (
                3,
                "solution5".to_string(),
                "signature5".to_string(),
                200,
                200
            ),
            (
                4,
                "solution6".to_string(),
                "signature6".to_string(),
                300,
                300
            ),
            (
                4,
                "solution7".to_string(),
                "signature7".to_string(),
                300,
                300
            ),
        ]
    );

    let result = query(
        &conn,
        include_sql!("query", "list_winning_batches"),
        named_params! {
            ":page_size": 2,
            ":page_number": 20,
        },
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
                row.get::<_, String>(2).unwrap(),
                row.get::<_, usize>(3).unwrap(),
                row.get::<_, usize>(4).unwrap(),
            )
        },
    );
    assert_eq!(
        result,
        vec![
            (
                41,
                "solution80".to_string(),
                "signature80".to_string(),
                40 * 100,
                40 * 100
            ),
            (
                41,
                "solution81".to_string(),
                "signature81".to_string(),
                40 * 100,
                40 * 100,
            ),
            (
                42,
                "solution82".to_string(),
                "signature82".to_string(),
                41 * 100,
                41 * 100,
            ),
            (
                42,
                "solution83".to_string(),
                "signature83".to_string(),
                41 * 100,
                41 * 100,
            ),
        ]
    );

    let result = query(
        &conn,
        include_sql!("query", "list_winning_batches_by_time"),
        named_params! {
            ":page_size": 1,
            ":page_number": 2,
            ":start_seconds": 40 * 100,
            ":start_nanos": 100,
            ":end_seconds": 100 * 100,
            ":end_nanos": 100 * 100,
        },
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
                row.get::<_, String>(2).unwrap(),
                row.get::<_, usize>(3).unwrap(),
                row.get::<_, usize>(4).unwrap(),
            )
        },
    );
    assert_eq!(
        result,
        vec![
            (
                43,
                "solution84".to_string(),
                "signature84".to_string(),
                42 * 100,
                42 * 100
            ),
            (
                43,
                "solution85".to_string(),
                "signature85".to_string(),
                42 * 100,
                42 * 100,
            ),
        ]
    );

    let result = query(
        &conn,
        include_sql!("query", "list_winning_batches_by_time"),
        named_params! {
            ":page_size": 1,
            ":page_number": 0,
            ":start_seconds": 41 * 100,
            ":start_nanos": 43 * 100,
            ":end_seconds": 100 * 100,
            ":end_nanos": 100 * 100,
        },
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
                row.get::<_, String>(2).unwrap(),
                row.get::<_, usize>(3).unwrap(),
                row.get::<_, usize>(4).unwrap(),
            )
        },
    );
    assert_eq!(
        result,
        vec![
            (
                43,
                "solution84".to_string(),
                "signature84".to_string(),
                42 * 100,
                42 * 100
            ),
            (
                43,
                "solution85".to_string(),
                "signature85".to_string(),
                42 * 100,
                42 * 100,
            ),
        ]
    );
}

fn move_solutions_to_solved(conn: &Connection, batch: usize, hashes: &[String], time: Duration) {
    conn.execute(
        include_sql!("insert", "batch"),
        params![
            format!("batch_hash{}", batch),
            time.as_secs(),
            time.subsec_nanos()
        ],
    )
    .unwrap();
    for hash in hashes {
        conn.execute(include_sql!("insert", "copy_to_solved"), [hash])
            .unwrap();
        conn.execute(include_sql!("update", "delete_from_solutions_pool"), [hash])
            .unwrap();
    }
}
