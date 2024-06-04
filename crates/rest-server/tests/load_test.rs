use std::{process::Stdio, sync::Arc, time::Duration};

use essential_constraint_vm::asm as constraint_asm;
use essential_rest_server::run;
use essential_rqlite_storage::RqliteStorage;
use essential_state_read_vm::asm as state_asm;
use essential_types::{
    intent::{Directive, Intent},
    solution::{Mutation, Solution, SolutionData},
    Block, IntentAddress, Word,
};
use rayon::prelude::*;
use reqwest::{Client, ClientBuilder, Version};
use tempfile::TempDir;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    task::JoinSet,
};

const NUM_SOLUTIONS: usize = 1_000;
const NUM_INTENTS: usize = 10;
const NUM_CONSTRAINTS: usize = 100;
const NUM_STATE: usize = 10;
const STATE_SIZE: usize = 32;
const KEY_SIZE: usize = 4;
const MAX_PARALLEL_REQUESTS: usize = 100;
const LEADER: &str = "entering leader state";

#[tokio::test(flavor = "multi_thread")]
#[ignore]
/// Requires rqlite be installed.
async fn load_test_the_server() {
    #[cfg(feature = "tracing")]
    tracing_subscriber::fmt::init();

    let start = std::time::Instant::now();

    // Spawn rqlited
    let tmp_dir = TempDir::new().unwrap();
    let mut child = Command::new("rqlited")
        .arg("-node-id")
        .arg("1")
        .arg("-raft-log-level")
        .arg("TRACE")
        .arg("-fk")
        .arg(&format!("{}", tmp_dir.path().display()))
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let stderr = child.stderr.take().unwrap();

    let buf = BufReader::new(stderr);
    let mut lines = buf.lines();

    // Wait till rqlite is ready
    loop {
        if let Some(line) = lines.next_line().await.unwrap() {
            if line.contains(LEADER) {
                break;
            }
        }
    }

    child.stderr = Some(lines.into_inner().into_inner());

    // Generate test data
    let (pk, _) = test_utils::random_keypair();

    let mut salt = (0..(NUM_SOLUTIONS * NUM_INTENTS)).map(|i| i as Word);

    println!("Generating test data...");
    let salts: Vec<_> = (0..NUM_SOLUTIONS)
        .map(|_| salt.by_ref().take(NUM_INTENTS).collect::<Vec<_>>())
        .collect();

    let sets = salts
        .into_par_iter()
        .map(|salt| {
            let set = create_intent(IntentConfig {
                salt,
                num_constraints: NUM_CONSTRAINTS,
                num_state: NUM_STATE,
                state_size: STATE_SIZE,
                key_size: KEY_SIZE,
            });
            essential_sign::intent_set::sign(set, &pk)
        })
        .collect::<Vec<_>>();

    let solutions: Vec<_> = sets
        .par_iter()
        .map(|set| {
            let set_addr = essential_hash::intent_set_addr::from_intents(&set.set);
            let configs: Vec<_> = set
                .set
                .iter()
                .map(|i| {
                    let addr = essential_hash::content_addr(i);
                    let intent = IntentAddress {
                        set: set_addr.clone(),
                        intent: addr,
                    };
                    SolutionConfig {
                        intent,
                        num_state: NUM_STATE,
                        state_size: STATE_SIZE,
                        key_size: KEY_SIZE,
                    }
                })
                .collect();
            create_solution(configs)
        })
        .collect();

    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup().await;

    println!("setup complete: {:?}", start.elapsed());

    let start = std::time::Instant::now();

    let mut join_set = JoinSet::new();

    // Deploy the intent sets
    let sem = Arc::new(tokio::sync::Semaphore::new(MAX_PARALLEL_REQUESTS));
    for set in sets {
        let permit = sem.clone().acquire_owned().await.unwrap();
        join_set.spawn({
            let client = client.clone();
            let url = url.clone();
            async move {
                let response = client
                    .post(url.join("/deploy-intent-set").unwrap())
                    .json(&set)
                    .version(Version::HTTP_2)
                    .send()
                    .await
                    .unwrap();
                assert_eq!(response.status(), 200, "{:?}", response.text().await);
                drop(permit);
            }
        });
    }

    while let Some(r) = join_set.join_next().await {
        r.unwrap();
    }

    println!("deployed : {:?}", start.elapsed());

    let start = std::time::Instant::now();

    let mut join_set = JoinSet::new();

    // Submit the solutions
    let sem = Arc::new(tokio::sync::Semaphore::new(MAX_PARALLEL_REQUESTS));
    for solution in &solutions {
        let permit = sem.clone().acquire_owned().await.unwrap();
        join_set.spawn({
            let client = client.clone();
            let url = url.clone();
            let solution = solution.clone();
            async move {
                let response = client
                    .post(url.join("/submit-solution").unwrap())
                    .json(&solution)
                    .version(Version::HTTP_2)
                    .send()
                    .await
                    .unwrap();
                assert_eq!(response.status(), 200, "{:?}", response.text().await);
                drop(permit);
            }
        });
    }

    while let Some(r) = join_set.join_next().await {
        r.unwrap();
    }

    println!("submitted solutions: {:?}", start.elapsed());

    let expecting = solutions.len();

    let start = std::time::Instant::now();
    let mut last_print = 0;

    // Wait for all solutions to be processed
    loop {
        let a = url.join("/list-winning-blocks").unwrap();
        let response = client.get(a).send().await.unwrap();
        assert_eq!(response.status(), 200);
        let blocks = response.json::<Vec<Block>>().await.unwrap();
        let mut total = 0;
        for block in &blocks {
            total += block.batch.solutions.len();
        }
        if total == expecting {
            println!("Got {} solutions", total);
            break;
        }
        last_print += 1;
        if last_print % 10 == 0 {
            println!("Solutions collected: {} out of {}", total, expecting);
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    println!("Blocks made: {:?}", start.elapsed());

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
    child.kill().await.unwrap();
}

static SERVER: &str = "127.0.0.1:0";
static CLIENT: &str = "http://127.0.0.1";
static DB: &str = "http://127.0.0.1:4001";

struct TestServer {
    client: Client,
    url: reqwest::Url,
    shutdown: tokio::sync::oneshot::Sender<()>,
    jh: tokio::task::JoinHandle<anyhow::Result<()>>,
}

#[derive(Clone, Debug)]
struct IntentConfig {
    salt: Vec<Word>,
    num_constraints: usize,
    num_state: usize,
    state_size: usize,
    key_size: usize,
}

#[derive(Clone, Debug)]
struct SolutionConfig {
    intent: IntentAddress,
    num_state: usize,
    state_size: usize,
    key_size: usize,
}

async fn setup() -> TestServer {
    let storage = RqliteStorage::new(DB)
        .await
        .expect("Failed to connect to rqlite");
    setup_with_rqlite(storage).await
}

async fn setup_with_rqlite(rqlite: RqliteStorage) -> TestServer {
    let config = Default::default();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let (shutdown, shutdown_rx) = tokio::sync::oneshot::channel();
    let server_config = essential_rest_server::Config {
        build_blocks: true,
        server_config: essential_server::Config {
            run_loop_interval: Duration::from_secs(1),
        },
    };
    let jh = tokio::task::spawn(async {
        let essential = essential_server::Essential::new(rqlite, config);
        run(essential, SERVER, tx, Some(shutdown_rx), server_config).await
    });
    let client = ClientBuilder::new()
        .http2_prior_knowledge()
        .build()
        .unwrap();
    let mut url = reqwest::Url::parse(CLIENT).unwrap();
    let port = rx.await.unwrap().port();
    url.set_port(Some(port)).unwrap();

    TestServer {
        client,
        url,
        shutdown,
        jh,
    }
}

fn create_intent(config: IntentConfig) -> Vec<Intent> {
    let IntentConfig {
        salt,
        num_constraints,
        num_state,
        state_size,
        key_size,
    } = config;
    salt.into_iter()
        .map(|salt| {
            let mut state_read: Vec<state_asm::Op> = vec![
                state_asm::Stack::Push(num_state as Word).into(),
                state_asm::StateSlots::AllocSlots.into(),
            ];
            let reads = (0..num_state).flat_map(|i| {
                let mut o = vec![state_asm::Op::from(state_asm::Stack::Push(i as Word)); key_size];
                o.extend([
                    state_asm::Op::from(state_asm::Stack::Push(key_size as Word)),
                    state_asm::Stack::Push(1).into(),
                    state_asm::Stack::Push(i as Word).into(),
                    state_asm::StateRead::KeyRange,
                ]);
                o
            });

            state_read.extend(reads);
            state_read.push(state_asm::ControlFlow::Halt.into());

            Intent {
                state_read: vec![state_asm::to_bytes(state_read).collect()],
                constraints: (0..num_constraints)
                    .map(|_| {
                        let mut c: Vec<constraint_asm::Op> = vec![
                            constraint_asm::Stack::Push(salt).into(),
                            constraint_asm::Stack::Pop.into(),
                        ];
                        let read_hash = (0..num_state).flat_map(|i| {
                            let o: [constraint_asm::Op; 12] = [
                                constraint_asm::Stack::Push(i as Word).into(), // slot
                                constraint_asm::Stack::Push(1).into(),         // post
                                constraint_asm::Access::State.into(),
                                constraint_asm::Stack::Push(state_size as Word).into(),
                                constraint_asm::Crypto::Sha256.into(),
                                constraint_asm::Stack::Push(i as Word).into(), // slot
                                constraint_asm::Stack::Push(1).into(),         // post
                                constraint_asm::Access::State.into(),
                                constraint_asm::Stack::Push(state_size as Word).into(),
                                constraint_asm::Crypto::Sha256.into(),
                                constraint_asm::Stack::Push(4).into(),
                                constraint_asm::Pred::EqRange.into(),
                            ];
                            o
                        });
                        c.extend(read_hash);
                        constraint_asm::to_bytes(c).collect()
                    })
                    .collect(),
                directive: Directive::Satisfy,
            }
        })
        .collect()
}

fn create_solution(config: Vec<SolutionConfig>) -> Solution {
    let data = config
        .iter()
        .map(|c| SolutionData {
            intent_to_solve: c.intent.clone(),
            decision_variables: vec![],
            state_mutations: (0..c.num_state)
                .map(|j| Mutation {
                    key: vec![j as Word; c.key_size],
                    value: vec![j as Word; c.state_size],
                })
                .collect(),
            transient_data: Default::default(),
        })
        .collect();
    Solution { data }
}
