use base64::Engine;
use core::time;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use essential_types::{intent::SignedSet, ContentAddress, Word};
use std::fs::read_dir;
use test_utils::{intent_with_salt, sign_intent_set_with_random_keypair};

use rusqlite::{params, Connection};

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

pub fn bench(c: &mut Criterion) {
    let conn = Connection::open_in_memory().unwrap();
    create_tables(&conn);
    let sets = (0..100).map(|i| {
        let mut set =
            sign_intent_set_with_random_keypair(vec![intent_with_salt(i), intent_with_salt(i + 1)]);
        set.set.sort_by_key(essential_hash::content_addr);
        set
    });

    let mut addresses = vec![];
    for set in sets {
        let address = insert_intent(&conn, set);
        addresses.push(address);
    }

    for i in 0..1000 {
        let key = [i as Word; 4];
        let value = [i as Word; 32];
        for address in &addresses {
            update_state(&conn, address, &key, &value);
        }
    }
    let mut i = (0..100).cycle();

    let key = [0 as Word; 4];
    let value = [0 as Word; 32];
    c.bench_function("update_state", |b| {
        b.iter(|| {
            let i = i.next().unwrap();
            update_state(&conn, &addresses[i], &key, &value);
        })
    });
    let key = [0 as Word; 4];
    let value = [0 as Word; 32];
    c.bench_function("query_state", |b| {
        b.iter(|| {
            query_state(&conn, &addresses[0], &key, &value);
        })
    });
    let mut set =
        sign_intent_set_with_random_keypair(vec![intent_with_salt(0), intent_with_salt(1)]);
    set.set.sort_by_key(essential_hash::content_addr);
    let sets = (100..10_000).map(|i| {
        let mut set =
            sign_intent_set_with_random_keypair(vec![intent_with_salt(i), intent_with_salt(i + 1)]);
        set.set.sort_by_key(essential_hash::content_addr);
        set
    });

    for set in sets {
        insert_intent(&conn, set);
    }

    c.bench_function("insert_intent", |b| {
        b.iter(|| {
            insert_intent(&conn, set.clone());
        })
    });

    c.bench_function("list_intent_sets", |b| {
        b.iter(|| {
            let r = query(
                &conn,
                include_sql!("query", "list_intent_sets"),
                [0, 100],
                |row| {
                    (
                        row.get::<_, u64>(0).unwrap(),
                        row.get::<_, String>(1).unwrap(),
                    )
                },
            );
            black_box(r);
        })
    });

    c.bench_function("get_intent_set", |b| {
        b.iter(|| {
            let r = query(
                &conn,
                include_sql!("query", "get_intent_set"),
                [encode(&addresses[0])],
                |row| (row.get::<_, String>(0).unwrap(),),
            );
            black_box(r);
        })
    });

    let intent_addr = essential_hash::content_addr(&set.set[0]);

    c.bench_function("get_intent", |b| {
        b.iter(|| {
            let r = query(
                &conn,
                include_sql!("query", "get_intent"),
                [encode(&addresses[0]), encode(&intent_addr)],
                |row| (row.get::<_, String>(0).unwrap(),),
            );
            black_box(r);
        })
    });
    // let path = concat!(env!("CARGO_MANIFEST_DIR"), "/my_db.sqlite");
    // // Connection::open(path).unwrap();
    // conn.backup(rusqlite::DatabaseName::Main, path, None).unwrap();
}

criterion_group!(benches, bench);
criterion_main!(benches);

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

/// Encodes a type into blob data which is then base64 encoded.
fn encode<T: serde::Serialize>(value: &T) -> String {
    let value = postcard::to_allocvec(value).expect("How can this fail?");
    base64::engine::general_purpose::STANDARD.encode(value)
}

/// Decodes a base64 encoded blob into a type.
fn decode<T: serde::de::DeserializeOwned>(value: &str) -> anyhow::Result<T> {
    let value = base64::engine::general_purpose::STANDARD.decode(value)?;
    Ok(postcard::from_bytes(&value)?)
}

fn insert_intent(conn: &Connection, set: SignedSet) -> ContentAddress {
    let set_address = essential_hash::intent_set_addr::from_intents(&set.set);
    let time = time::Duration::from_secs(1);
    let address = encode(&set_address);
    conn.execute(
        include_sql!("insert", "intent_set"),
        params![
            address.clone(),
            encode(&set.signature),
            time.as_secs(),
            time.subsec_nanos()
        ],
    )
    .unwrap();

    for intent in &set.set {
        let hash = encode(&essential_hash::content_addr(&intent));
        conn.execute(
            include_sql!("insert", "intents"),
            [encode(intent), hash.clone()],
        )
        .unwrap();
        conn.execute(
            include_sql!("insert", "intent_set_pairing"),
            params![address.clone(), hash,],
        )
        .unwrap();
    }
    set_address
}

fn update_state(conn: &Connection, address: &ContentAddress, key: &[Word], value: &[Word]) {
    conn.execute(
        include_sql!("update", "update_state"),
        params![encode(&key), encode(&value), encode(&address)],
    )
    .unwrap();
}

fn query_state(conn: &Connection, address: &ContentAddress, key: &[Word], value: &[Word]) {
    let r = query(
        conn,
        include_sql!("query", "get_state"),
        [encode(&address), encode(&key)],
        |row| row.get::<_, String>(0).unwrap(),
    );
    let v: Vec<Word> = decode(&r[0]).unwrap();
    assert_eq!(v, *value);
}

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
