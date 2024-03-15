#![allow(dead_code)]

use std::{fs::read_dir, ops::Range, time::Duration};

use rusqlite::{params, Connection};

pub fn create_tables(conn: &Connection) {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/sql/create/");
    for entry in read_dir(path).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() && entry.path().extension().unwrap() == "sql" {
            let sql = std::fs::read_to_string(entry.path()).unwrap();
            conn.execute(&sql, []).unwrap();
        }
    }
}

macro_rules! include_sql {
    ($dir:expr, $sql:expr) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/sql/",
            $dir,
            "/",
            $sql,
            ".sql"
        ))
    };
}

pub(crate) use include_sql;

pub fn query<P, F, R>(conn: &Connection, sql: &str, params: P, mut f: F) -> Vec<R>
where
    P: rusqlite::Params,
    F: FnMut(&rusqlite::Row) -> R,
{
    conn.prepare(sql)
        .unwrap()
        .query_map(params, |row| Ok(f(row)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect()
}

pub fn insert_intent_set(conn: &Connection, set: usize, unix_time: Duration, range: Range<usize>) {
    conn.execute(
        include_sql!("insert", "intent_set"),
        params![
            format!("hash{}", set),
            format!("signature{}", set),
            unix_time.as_secs(),
            unix_time.subsec_nanos()
        ],
    )
    .unwrap();

    for i in range {
        conn.execute(
            include_sql!("insert", "intents"),
            params![format!("intent{}", i), format!("intent_hash{}", i),],
        )
        .unwrap();
        conn.execute(
            include_sql!("insert", "intent_set_pairing"),
            params![format!("hash{}", set), format!("intent_hash{}", i),],
        )
        .unwrap();
    }
}
