use core::time;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use essential_storage::failed_solution::SolutionFailReason;
use essential_types::{intent::SignedSet, solution::Solution, ContentAddress, Hash, Word};
use std::fs::read_dir;
use test_utils::{
    intent_with_salt, sign_intent_set_with_random_keypair, solution_with_all_inputs_fixed_size,
};

use rusqlite::{named_params, params, Connection};

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

    c.bench_function("delete_state", |b| {
        b.iter(|| {
            let i = i.next().unwrap();
            delete_state(&conn, &addresses[i], &key);
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

    c.bench_function("list_intent_sets_by_time", |b| {
        b.iter(|| {
            let r = query(
                &conn,
                include_sql!("query", "list_intent_sets_by_time"),
                named_params! {
                    ":page_size": 100,
                    ":page_number": 0,
                    ":start_seconds": 0,
                    ":start_nanos": 0,
                    ":end_seconds": 10000,
                    ":end_nanos": 0,
                },
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

    let num_solutions = 10_000;

    let solutions = (0..num_solutions).map(|i| solution_with_all_inputs_fixed_size(i, 10));
    let mut s_hashes = vec![];
    for solution in solutions {
        let hash = insert_solution(&conn, &solution);
        s_hashes.push(hash);
    }
    let solution = solution_with_all_inputs_fixed_size(0, 1000);
    c.bench_function("insert_solution", |b| {
        b.iter(|| {
            insert_solution(&conn, &solution);
        })
    });

    let reason =
        SolutionFailReason::ConstraintsFailed("This failed because of some reason".to_string());
    let hash = essential_hash::hash(&solution);
    c.bench_function("copy_to_failed", |b| {
        b.iter(|| {
            insert_solution(&conn, &solution);
            conn.execute(
                include_sql!("insert", "copy_to_failed"),
                params![encode(&reason), 0, 0, encode(&hash),],
            )
            .unwrap();
            conn.execute(
                include_sql!("update", "delete_from_solutions_pool"),
                [encode(&hash)],
            )
            .unwrap();
        })
    });

    c.bench_function("copy_to_solved", |b| {
        b.iter(|| {
            insert_solution(&conn, &solution);
            conn.execute(include_sql!("insert", "batch"), [0, 0])
                .unwrap();
            conn.execute(include_sql!("insert", "copy_to_solved"), [encode(&hash)])
                .unwrap();
            conn.execute(
                include_sql!("update", "delete_from_solutions_pool"),
                [encode(&hash)],
            )
            .unwrap();
            conn.execute(include_sql!("update", "delete_empty_batch"), [])
                .unwrap();
        })
    });

    let i = (0..num_solutions).cycle();
    for i in i.take(1_000) {
        conn.execute(include_sql!("insert", "batch"), [0, 0])
            .unwrap();
        conn.execute(
            include_sql!("insert", "copy_to_solved"),
            [encode(&s_hashes[i])],
        )
        .unwrap();
    }
    let i = (0..num_solutions).cycle();
    for i in i.take(1_000) {
        conn.execute(
            include_sql!("insert", "copy_to_failed"),
            params![encode(&reason), 0, 0, encode(&s_hashes[i]),],
        )
        .unwrap();
    }

    let mut i = (0..num_solutions).cycle();
    c.bench_function("get_solution_outcomes", |b| {
        b.iter(|| {
            let i = i.next().unwrap();
            let r = query(
                &conn,
                include_sql!("query", "get_solution_outcomes"),
                [encode(&s_hashes[i]), encode(&s_hashes[i])],
                |row| {
                    (
                        row.get::<_, Option<u64>>(0).unwrap(),
                        row.get::<_, Option<String>>(1).unwrap(),
                        row.get::<_, u64>(2).unwrap(),
                        row.get::<_, u64>(3).unwrap(),
                    )
                },
            );
            black_box(r);
        })
    });

    c.bench_function("get_solution_query", |b| {
        b.iter(|| {
            let i = i.next().unwrap();
            let r = query(
                &conn,
                include_sql!("query", "get_solution"),
                [encode(&s_hashes[i])],
                |row| (row.get::<_, String>(0).unwrap(),),
            );
            black_box(r);
        })
    });

    // This is slow but it really depends on the page size and number.
    c.bench_function("list_failed_solutions", |b| {
        b.iter(|| {
            let r = query(
                &conn,
                include_sql!("query", "list_failed_solutions"),
                named_params! {
                    ":page_size": 100,
                    ":page_number": 2,
                },
                |row| {
                    (
                        row.get::<_, String>(0).unwrap(),
                        row.get::<_, String>(1).unwrap(),
                    )
                },
            );
            black_box(r);
        })
    });

    c.bench_function("list_solutions_pool", |b| {
        b.iter(|| {
            let r = query(
                &conn,
                include_sql!("query", "list_solutions_pool"),
                named_params! {
                    ":page_size": 100,
                    ":page_number": 2,
                },
                |row| (row.get::<_, String>(0).unwrap(),),
            );
            black_box(r);
        })
    });

    c.bench_function("list_winning_batches_query", |b| {
        b.iter(|| {
            let r = query(
                &conn,
                include_sql!("query", "list_winning_batches"),
                named_params! {
                    ":page_size": 100,
                    ":page_number": 2,
                },
                |row| {
                    (
                        row.get::<_, u64>(0).unwrap(),
                        row.get::<_, String>(1).unwrap(),
                        row.get::<_, u64>(2).unwrap(),
                        row.get::<_, u64>(3).unwrap(),
                    )
                },
            );
            black_box(r);
        })
    });

    c.bench_function("list_winning_batches_by_time", |b| {
        b.iter(|| {
            let r = query(
                &conn,
                include_sql!("query", "list_winning_batches_by_time"),
                named_params! {
                    ":page_size": 100,
                    ":page_number": 2,
                    ":start_seconds": 0,
                    ":start_nanos": 0,
                    ":end_seconds": 10000,
                    ":end_nanos": 0,
                },
                |row| {
                    (
                        row.get::<_, u64>(0).unwrap(),
                        row.get::<_, String>(1).unwrap(),
                        row.get::<_, u64>(2).unwrap(),
                        row.get::<_, u64>(3).unwrap(),
                    )
                },
            );
            black_box(r);
        })
    });

    c.bench_function("prune_failed", |b| {
        b.iter(|| {
            conn.execute(include_sql!("update", "prune_failed"), [100000])
                .unwrap();
        })
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);

fn create_tables(conn: &Connection) {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/sql/create/");
    create(path, conn);
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/sql/index/");
    create(path, conn);
}

fn create(path: &str, conn: &Connection) {
    for entry in read_dir(path).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() && entry.path().extension().unwrap() == "sql" {
            let sql = std::fs::read_to_string(entry.path()).unwrap();
            conn.execute(&sql, []).unwrap();
        }
    }
}

/// Encodes a type into blob data which is then hex encoded.
fn encode<T: serde::Serialize>(value: &T) -> String {
    let value = postcard::to_allocvec(value).expect("How can this fail?");
    hex::encode_upper(value)
}

/// Decodes a hex encoded blob into a type.
fn decode<T: serde::de::DeserializeOwned>(value: &str) -> anyhow::Result<T> {
    let value = hex::decode(value)?;
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

fn insert_solution(conn: &Connection, solution: &Solution) -> Hash {
    let hash = essential_hash::hash(solution);
    let h = encode(&hash);
    conn.execute(
        include_sql!("insert", "solutions"),
        params![h.clone(), encode(solution)],
    )
    .unwrap();
    conn.execute(include_sql!("insert", "solutions_pool"), params![h.clone()])
        .unwrap();
    hash
}

fn update_state(conn: &Connection, address: &ContentAddress, key: &[Word], value: &[Word]) {
    conn.execute(
        include_sql!("update", "update_state"),
        params![encode(&key), encode(&value), encode(&address)],
    )
    .unwrap();
}

fn delete_state(conn: &Connection, address: &ContentAddress, key: &[Word]) {
    conn.execute(
        include_sql!("update", "delete_state"),
        params![encode(&address), encode(&key)],
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
