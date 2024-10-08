use std::time::Duration;

use rusqlite::{named_params, params, Connection};

use common::*;

mod common;

#[test]
fn test_insert_solutions() {
    let conn = Connection::open_in_memory().unwrap();
    create_tables(&conn);

    // Double insert is a noop
    conn.execute(include_sql!("insert", "solutions"), ["hash1", "solution1"])
        .unwrap();
    conn.execute(include_sql!("insert", "solutions_pool"), ["hash1"])
        .unwrap();

    conn.execute(include_sql!("insert", "solutions"), ["hash1", "solution1"])
        .unwrap();
    conn.execute(include_sql!("insert", "solutions_pool"), ["hash1"])
        .unwrap();

    let result = query(&conn, "select * from solutions_pool", [], |row| {
        (
            row.get::<_, usize>(0).unwrap(),
            row.get::<_, String>(1).unwrap(),
        )
    });
    assert_eq!(result, vec![(1, "hash1".to_string())]);
    let result = query(&conn, "select * from solutions", [], |row| {
        (
            row.get::<_, usize>(0).unwrap(),
            row.get::<_, String>(1).unwrap(),
            row.get::<_, String>(2).unwrap(),
        )
    });
    assert_eq!(
        result,
        vec![(1, "hash1".to_string(), "solution1".to_string(),)]
    );

    // Can insert a second solution
    conn.execute(include_sql!("insert", "solutions"), ["hash2", "solution2"])
        .unwrap();
    conn.execute(include_sql!("insert", "solutions_pool"), ["hash2"])
        .unwrap();

    let result = query(&conn, "select * from solutions_pool", [], |row| {
        (
            row.get::<_, usize>(0).unwrap(),
            row.get::<_, String>(1).unwrap(),
        )
    });
    assert_eq!(
        result,
        vec![(1, "hash1".to_string(),), (2, "hash2".to_string(),),]
    );
    let result = query(&conn, "select * from solutions", [], |row| {
        (
            row.get::<_, usize>(0).unwrap(),
            row.get::<_, String>(1).unwrap(),
            row.get::<_, String>(2).unwrap(),
        )
    });
    assert_eq!(
        result,
        vec![
            (1, "hash1".to_string(), "solution1".to_string(),),
            (2, "hash2".to_string(), "solution2".to_string(),),
        ]
    );

    // list solutions pool
    let result = query(
        &conn,
        include_sql!("query", "list_solutions_pool"),
        named_params! {
            ":page_size": 10,
            ":page_number": 0,
        },
        |row| (row.get::<_, String>(0).unwrap(),),
    );
    assert_eq!(
        result,
        vec![("solution1".to_string(),), ("solution2".to_string(),),]
    );

    // Move solutions to solved
    conn.execute(include_sql!("insert", "batch"), params![0, 0])
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
        named_params! {
            ":page_size": 10,
            ":page_number": 0,
        },
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
            ":block_number": 0,
            ":page_size": 10,
            ":page_number": 0,
        },
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
                row.get::<_, usize>(2).unwrap(),
                row.get::<_, usize>(3).unwrap(),
            )
        },
    );

    assert_eq!(
        result,
        vec![
            (1, "solution1".to_string(), 0, 0),
            (1, "solution2".to_string(), 0, 0),
        ]
    );

    let result = query(
        &conn,
        include_sql!("query", "get_solution"),
        ["hash1"],
        |row| row.get::<_, String>(0).unwrap(),
    );

    assert_eq!(result, vec!["solution1".to_string()]);

    let result = query(
        &conn,
        include_sql!("query", "get_solution_outcomes"),
        ["hash1", "hash1"],
        |row| {
            (
                row.get::<_, Option<u64>>(0).unwrap(),
                row.get::<_, Option<String>>(1).unwrap(),
            )
        },
    );

    assert_eq!(result, vec![(Some(1), None,),]);
}

#[test]
fn test_list_solutions_pool_page() {
    let conn = Connection::open_in_memory().unwrap();
    create_tables(&conn);

    for i in 0..10 {
        conn.execute(
            include_sql!("insert", "solutions"),
            [format!("hash{}", i), format!("solution{}", i)],
        )
        .unwrap();
        conn.execute(
            include_sql!("insert", "solutions_pool"),
            [format!("hash{}", i)],
        )
        .unwrap();
    }

    let result = query(
        &conn,
        include_sql!("query", "list_solutions_pool"),
        named_params! {
            ":page_size": 1,
            ":page_number": 0,
        },
        |row| (row.get::<_, String>(0).unwrap(),),
    );

    assert_eq!(result, vec![("solution0".to_string(),)]);

    let result = query(
        &conn,
        include_sql!("query", "list_solutions_pool"),
        named_params! {
            ":page_size": 1,
            ":page_number": 1,
        },
        |row| (row.get::<_, String>(0).unwrap(),),
    );

    assert_eq!(result, vec![("solution1".to_string(),)]);

    let result = query(
        &conn,
        include_sql!("query", "list_solutions_pool"),
        named_params! {
            ":page_size": 2,
            ":page_number": 1,
        },
        |row| (row.get::<_, String>(0).unwrap(),),
    );

    assert_eq!(
        result,
        vec![("solution2".to_string(),), ("solution3".to_string(),)]
    );

    let result = query(
        &conn,
        include_sql!("query", "list_solutions_pool"),
        named_params! {
            ":page_size": 3,
            ":page_number": 3,
        },
        |row| (row.get::<_, String>(0).unwrap(),),
    );

    assert_eq!(result, vec![("solution9".to_string(),)]);
}

#[test]
fn test_move_solutions_to_failed() {
    let conn = Connection::open_in_memory().unwrap();
    create_tables(&conn);

    conn.execute(include_sql!("insert", "solutions"), ["hash1", "solution1"])
        .unwrap();

    conn.execute(include_sql!("insert", "solutions"), ["hash2", "solution2"])
        .unwrap();

    conn.execute(include_sql!("insert", "solutions_pool"), ["hash1"])
        .unwrap();

    conn.execute(include_sql!("insert", "solutions_pool"), ["hash2"])
        .unwrap();

    move_solutions_to_failed(&conn, &[("hash1", "reason1", 10), ("hash2", "reason2", 20)]);

    // pool is empty
    let result = query(
        &conn,
        include_sql!("query", "list_solutions_pool"),
        named_params! {
            ":page_size": 10,
            ":page_number": 0,
        },
        |row| (row.get::<_, String>(0).unwrap(),),
    );
    assert_eq!(result, vec![]);

    let result = query(
        &conn,
        include_sql!("query", "list_failed_solutions"),
        named_params! {
            ":page_size": 10,
            ":page_number": 0,
        },
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
            ("solution1".to_string(), "reason1".to_string(),),
            ("solution2".to_string(), "reason2".to_string(),),
        ]
    );

    let result = query(
        &conn,
        include_sql!("query", "get_solution"),
        ["hash1"],
        |row| row.get::<_, String>(0).unwrap(),
    );

    assert_eq!(result, vec!["solution1".to_string()]);

    let result = query(
        &conn,
        include_sql!("query", "get_solution_outcomes"),
        ["hash1", "hash1"],
        |row| {
            (
                row.get::<_, Option<u64>>(0).unwrap(),
                row.get::<_, Option<String>>(1).unwrap(),
            )
        },
    );

    assert_eq!(result, vec![(None, Some("reason1".to_string()),),]);

    conn.execute(include_sql!("update", "prune_failed"), [15])
        .unwrap();

    let result = query(
        &conn,
        include_sql!("query", "list_failed_solutions"),
        named_params! {
            ":page_size": 10,
            ":page_number": 0,
        },
        |row| {
            (
                row.get::<_, String>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
            )
        },
    );

    assert_eq!(
        result,
        vec![("solution2".to_string(), "reason2".to_string(),),]
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
                include_sql!("insert", "solutions"),
                [hash.to_string(), format!("solution{}", i)],
            )
            .unwrap();
            conn.execute(include_sql!("insert", "solutions_pool"), [hash.to_string()])
                .unwrap();
        }

        move_solutions_to_solved(
            &conn,
            &hashes,
            Duration::new((100 * n) as u64, (100 * n) as u32),
        );
    }

    let result = query(
        &conn,
        include_sql!("query", "list_winning_batches"),
        named_params! {
            ":block_number": 0,
            ":page_size": 2,
            ":page_number": 0,
        },
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
                row.get::<_, usize>(2).unwrap(),
                row.get::<_, usize>(3).unwrap(),
            )
        },
    );
    assert_eq!(
        result,
        vec![
            (1, "solution0".to_string(), 0, 0),
            (1, "solution1".to_string(), 0, 0),
            (2, "solution2".to_string(), 100, 100),
            (2, "solution3".to_string(), 100, 100),
        ]
    );

    let result = query(
        &conn,
        include_sql!("query", "list_winning_batches"),
        named_params! {
            ":block_number": 0,
            ":page_size": 2,
            ":page_number": 1,
        },
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
                row.get::<_, usize>(2).unwrap(),
                row.get::<_, usize>(3).unwrap(),
            )
        },
    );
    assert_eq!(
        result,
        vec![
            (3, "solution4".to_string(), 200, 200),
            (3, "solution5".to_string(), 200, 200),
            (4, "solution6".to_string(), 300, 300),
            (4, "solution7".to_string(), 300, 300),
        ]
    );

    let result = query(
        &conn,
        include_sql!("query", "list_winning_batches"),
        named_params! {
            ":block_number": 0,
            ":page_size": 2,
            ":page_number": 20,
        },
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
                row.get::<_, usize>(2).unwrap(),
                row.get::<_, usize>(3).unwrap(),
            )
        },
    );
    assert_eq!(
        result,
        vec![
            (41, "solution80".to_string(), 40 * 100, 40 * 100),
            (41, "solution81".to_string(), 40 * 100, 40 * 100,),
            (42, "solution82".to_string(), 41 * 100, 41 * 100,),
            (42, "solution83".to_string(), 41 * 100, 41 * 100,),
        ]
    );

    let result = query(
        &conn,
        include_sql!("query", "list_winning_batches"),
        named_params! {
            ":block_number": 10,
            ":page_size": 2,
            ":page_number": 0,
        },
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
                row.get::<_, usize>(2).unwrap(),
                row.get::<_, usize>(3).unwrap(),
            )
        },
    );
    assert_eq!(
        result,
        vec![
            (11, "solution20".to_string(), 10 * 100, 10 * 100),
            (11, "solution21".to_string(), 10 * 100, 10 * 100,),
            (12, "solution22".to_string(), 11 * 100, 11 * 100,),
            (12, "solution23".to_string(), 11 * 100, 11 * 100,),
        ]
    );

    let result = query(
        &conn,
        include_sql!("query", "list_winning_batches"),
        named_params! {
            ":block_number": 10,
            ":page_size": 2,
            ":page_number": 1,
        },
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
                row.get::<_, usize>(2).unwrap(),
                row.get::<_, usize>(3).unwrap(),
            )
        },
    );
    assert_eq!(
        result,
        vec![
            (13, "solution24".to_string(), 12 * 100, 12 * 100),
            (13, "solution25".to_string(), 12 * 100, 12 * 100,),
            (14, "solution26".to_string(), 13 * 100, 13 * 100,),
            (14, "solution27".to_string(), 13 * 100, 13 * 100,),
        ]
    );

    let result = query(
        &conn,
        include_sql!("query", "list_winning_batches_by_time"),
        named_params! {
            ":block_number": 40,
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
                row.get::<_, usize>(2).unwrap(),
                row.get::<_, usize>(3).unwrap(),
            )
        },
    );
    assert_eq!(
        result,
        vec![
            (43, "solution84".to_string(), 42 * 100, 42 * 100),
            (43, "solution85".to_string(), 42 * 100, 42 * 100,),
        ]
    );

    let result = query(
        &conn,
        include_sql!("query", "list_winning_batches_by_time"),
        named_params! {
            ":block_number": 44,
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
                row.get::<_, usize>(2).unwrap(),
                row.get::<_, usize>(3).unwrap(),
            )
        },
    );
    assert_eq!(
        result,
        vec![
            (47, "solution92".to_string(), 46 * 100, 46 * 100),
            (47, "solution93".to_string(), 46 * 100, 46 * 100,),
        ]
    );

    let result = query(
        &conn,
        include_sql!("query", "list_winning_batches_by_time"),
        named_params! {
            ":block_number": 0,
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
                row.get::<_, usize>(2).unwrap(),
                row.get::<_, usize>(3).unwrap(),
            )
        },
    );
    assert_eq!(
        result,
        vec![
            (43, "solution84".to_string(), 42 * 100, 42 * 100),
            (43, "solution85".to_string(), 42 * 100, 42 * 100,),
        ]
    );

    let result = query(
        &conn,
        include_sql!("query", "get_latest_block"),
        [],
        |row| {
            (
                row.get::<_, usize>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
                row.get::<_, usize>(2).unwrap(),
                row.get::<_, usize>(3).unwrap(),
            )
        },
    );
    assert_eq!(
        result,
        vec![
            (1000, "solution1998".to_string(), 99900, 99900),
            (1000, "solution1999".to_string(), 99900, 99900),
        ]
    );
}

#[test]
fn test_empty_batch() {
    let conn = Connection::open_in_memory().unwrap();
    create_tables(&conn);

    for i in 0..4 {
        conn.execute(
            include_sql!("insert", "solutions"),
            [&format!("hash{}", i), "solution1"],
        )
        .unwrap();

        conn.execute(
            include_sql!("insert", "solutions_pool"),
            [&format!("hash{}", i)],
        )
        .unwrap();
    }

    let time = Duration::new(0, 0);
    move_solutions_to_solved(&conn, &["hash0".to_string(), "hash1".to_string()], time);
    let time = Duration::new(1, 1);
    move_solutions_to_solved(&conn, &[], time);
    let time = Duration::new(2, 2);
    move_solutions_to_solved(&conn, &["hash2".to_string(), "hash3".to_string()], time);
    let result = query(&conn, "select id from batch", [], |row| {
        row.get::<_, usize>(0).unwrap()
    });
    assert_eq!(result, vec![1, 2]);
}

fn move_solutions_to_solved(conn: &Connection, hashes: &[String], time: Duration) {
    conn.execute(
        include_sql!("insert", "batch"),
        params![time.as_secs(), time.subsec_nanos()],
    )
    .unwrap();
    for hash in hashes {
        conn.execute(include_sql!("insert", "copy_to_solved"), [hash])
            .unwrap();
        conn.execute(include_sql!("update", "delete_from_solutions_pool"), [hash])
            .unwrap();
    }
    conn.execute(include_sql!("update", "delete_empty_batch"), [])
        .unwrap();
}

fn move_solutions_to_failed(conn: &Connection, hashes_reasons: &[(&str, &str, u64)]) {
    for (hash, reason, secs) in hashes_reasons {
        conn.execute(
            include_sql!("insert", "copy_to_failed"),
            params![reason, secs, 0, hash],
        )
        .unwrap();
        conn.execute(include_sql!("update", "delete_from_solutions_pool"), [hash])
            .unwrap();
    }
}

#[test]
fn test_ser() {
    let json = r#"{
        "results": [
            {
                "columns": [
                    "a",
                    "b"
                ],
                "types": [
                    "blob",
                    ""
                ],
                "values": [
                    [
                        "apple",
                        null
                    ],
                    [
                        null,
                        "banana"
                    ]
                ]
            }
        ]
    }"#;
    let _v: serde_json::Value = serde_json::from_str(json).unwrap();
}
