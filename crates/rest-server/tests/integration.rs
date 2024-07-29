use std::{time::Duration, vec};

use essential_memory_storage::MemoryStorage;
use essential_server::{CheckSolutionOutput, SolutionOutcome};
use essential_server_types::{
    CheckSolution, QueryStateReads, QueryStateReadsOutput, Slots, StateReadRequestType,
};
use essential_storage::{StateStorage, Storage};
use essential_types::{
    contract::{Contract, SignedContract},
    convert::{bytes_from_word, word_4_from_u8_32},
    predicate::Predicate,
    solution::{Solution, SolutionData},
    Block, ContentAddress, PredicateAddress, Word,
};
use futures::{StreamExt, TryStreamExt};
use test_utils::{
    empty::Empty, sign_contract_with_random_keypair, solution_with_all_inputs_fixed_size,
    solution_with_decision_variables,
};
use tokio_util::bytes::Buf;
use tokio_util::{
    bytes,
    codec::{Decoder, FramedRead},
    io::StreamReader,
};
use utils::{setup, setup_with_mem, TestServer};

mod utils;

#[tokio::test]
async fn test_deploy_contract() {
    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup().await;

    let contract = sign_contract_with_random_keypair(vec![Predicate::empty()]);
    let response = client
        .post(url.join("/deploy-contract").unwrap())
        .json(&contract)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let address = response.json::<ContentAddress>().await.unwrap();
    let expected = essential_hash::contract_addr::from_contract(&contract.contract);
    assert_eq!(address, expected);

    let a = url.join(&format!("/get-contract/{address}")).unwrap();
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let contract = response
        .json::<Option<SignedContract>>()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(contract, contract);

    let predicate_address = PredicateAddress {
        contract: address,
        predicate: essential_hash::content_addr(&contract.contract[0]),
    };
    let a = url
        .join(&format!(
            "/get-predicate/{}/{}",
            predicate_address.contract, predicate_address.predicate,
        ))
        .unwrap();
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let predicate = response.json::<Option<Predicate>>().await.unwrap().unwrap();

    assert_eq!(predicate, contract.contract[0]);

    let mut a = url.join("/list-contracts").unwrap();
    let time = std::time::UNIX_EPOCH.elapsed().unwrap() + Duration::from_secs(600);
    a.query_pairs_mut()
        .append_pair("start", "0")
        .append_pair("end", time.as_secs().to_string().as_str())
        .append_pair("page", "0");
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let contracts = response.json::<Vec<Contract>>().await.unwrap();
    assert_eq!(contract.contract, contracts[0].clone());

    let a = url.join("/list-contracts").unwrap();
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let contracts = response.json::<Vec<Contract>>().await.unwrap();
    assert_eq!(contract.contract, contracts[0].clone());

    let mut a = url.join("/list-contracts").unwrap();
    a.query_pairs_mut().append_pair("page", "1");
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let contract = response.json::<Vec<Contract>>().await.unwrap();
    assert!(contract.is_empty());

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}

#[tokio::test]
async fn test_submit_solution() {
    let predicate = Predicate::empty();
    let predicate_addr = essential_hash::content_addr(&predicate);
    let contract = sign_contract_with_random_keypair(vec![predicate]);
    let contract_addr = essential_hash::contract_addr::from_contract(&contract.contract);

    let mem = MemoryStorage::new();
    mem.insert_contract(contract).await.unwrap();

    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup_with_mem(mem).await;
    let mut solution = solution_with_decision_variables(1);
    solution.data[0].predicate_to_solve = PredicateAddress {
        contract: contract_addr,
        predicate: predicate_addr,
    };
    let response = client
        .post(url.join("/submit-solution").unwrap())
        .json(&solution)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let ca = response
        .json::<essential_types::ContentAddress>()
        .await
        .unwrap();
    assert_eq!(ca, essential_hash::content_addr(&solution));

    let response = client
        .get(url.join("list-solutions-pool").unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let solutions = response.json::<Vec<Solution>>().await.unwrap();

    assert_eq!(solutions.len(), 1);
    assert_eq!(essential_hash::content_addr(&solutions[0]), ca);

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}

#[tokio::test]
async fn test_query_state() {
    let contract = sign_contract_with_random_keypair(vec![Predicate::empty()]);
    let address = essential_hash::contract_addr::from_contract(&contract.contract);
    let key = vec![0; 4];

    let mem = MemoryStorage::new();
    mem.insert_contract(contract).await.unwrap();
    mem.update_state(&address, &key, vec![42]).await.unwrap();

    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup_with_mem(mem).await;

    let a = url
        .join(&format!(
            "/query-state/{address}/{}",
            hex::encode_upper(
                key.into_iter()
                    .flat_map(bytes_from_word)
                    .collect::<Vec<u8>>()
            ),
        ))
        .unwrap();
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let value = response.json::<Vec<Word>>().await.unwrap();

    assert_eq!(value, vec![42]);

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}

#[tokio::test]
async fn test_query_state_reads() {
    let contract = sign_contract_with_random_keypair(vec![Predicate::empty()]);
    let address = essential_hash::contract_addr::from_contract(&contract.contract);
    let addr_words = word_4_from_u8_32(address.0);

    let read_key: Vec<u8> = essential_state_read_vm::asm::to_bytes(vec![
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::StateSlots::AllocSlots.into(),
        essential_state_read_vm::asm::Stack::Push(addr_words[0]).into(),
        essential_state_read_vm::asm::Stack::Push(addr_words[1]).into(),
        essential_state_read_vm::asm::Stack::Push(addr_words[2]).into(),
        essential_state_read_vm::asm::Stack::Push(addr_words[3]).into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(1).into(), // key length
        essential_state_read_vm::asm::Stack::Push(1).into(), // num values to read
        essential_state_read_vm::asm::Stack::Push(0).into(), // slot index
        essential_state_read_vm::asm::StateRead::KeyRangeExtern,
        essential_state_read_vm::asm::TotalControlFlow::Halt.into(),
    ])
    .collect();

    let query = QueryStateReads::inline_empty(vec![read_key], StateReadRequestType::default());

    let mem = MemoryStorage::new();
    mem.insert_contract(contract).await.unwrap();
    mem.update_state(&address, &vec![0], vec![42])
        .await
        .unwrap();

    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup_with_mem(mem).await;

    let response = client
        .post(url.join("/query-state-reads").unwrap())
        .json(&query)
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        200,
        "response: {}",
        response.text().await.unwrap()
    );
    let outcome = response.json::<QueryStateReadsOutput>().await.unwrap();

    let expect = QueryStateReadsOutput::All(
        [(address.clone(), [(vec![0], vec![42])].into_iter().collect())]
            .into_iter()
            .collect(),
        Slots {
            pre: vec![vec![42]],
            post: vec![vec![42]],
        },
    );
    assert_eq!(outcome, expect);

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}

#[tokio::test]
async fn test_list_winning_blocks() {
    let solution = Solution::empty();
    let hash = essential_hash::hash(&solution);

    let mem = MemoryStorage::new();
    mem.insert_solution_into_pool(solution).await.unwrap();
    mem.move_solutions_to_solved(&[hash]).await.unwrap();

    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup_with_mem(mem).await;

    let mut a = url.join("/list-blocks").unwrap();
    a.query_pairs_mut().append_pair("page", "0");
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let blocks = response.json::<Vec<Block>>().await.unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(essential_hash::hash(&blocks[0].solutions[0]), hash);

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}

#[tokio::test]
async fn test_solution_outcome() {
    let solution = Solution::empty();
    let ca = essential_hash::content_addr(&solution);

    let mem = MemoryStorage::new();
    mem.insert_solution_into_pool(solution).await.unwrap();
    mem.move_solutions_to_solved(&[ca.0]).await.unwrap();

    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup_with_mem(mem).await;

    let a = url.join(&format!("/solution-outcome/{ca}")).unwrap();
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let value = response.json::<Vec<SolutionOutcome>>().await.unwrap();

    assert_eq!(value, vec![SolutionOutcome::Success(0)]);

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}

#[tokio::test]
async fn test_check_solution() {
    let contract = sign_contract_with_random_keypair(vec![Predicate::empty()]);
    let mem = MemoryStorage::new();
    mem.insert_contract(contract.clone()).await.unwrap();

    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup_with_mem(mem).await;

    let contract_addr = essential_hash::contract_addr::from_contract(&contract.contract);
    let address = essential_hash::content_addr(&contract.contract[0]);
    let mut solution = Solution::empty();
    solution.data.push(SolutionData {
        predicate_to_solve: PredicateAddress {
            contract: contract_addr,
            predicate: address,
        },
        decision_variables: vec![],
        state_mutations: vec![],
        transient_data: vec![],
    });
    let response = client
        .post(url.join("/check-solution").unwrap())
        .json(&solution)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let value = response
        .json::<Option<CheckSolutionOutput>>()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        value,
        CheckSolutionOutput {
            utility: 1.0,
            gas: 0
        }
    );

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}

#[tokio::test]
async fn test_check_solution_with_data() {
    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup().await;

    let contract = vec![Predicate::empty()].into();
    let contract_addr = essential_hash::contract_addr::from_contract(&contract);
    let address = essential_hash::content_addr(&contract[0]);
    let mut solution = Solution::empty();
    solution.data.push(SolutionData {
        predicate_to_solve: PredicateAddress {
            contract: contract_addr.clone(),
            predicate: address,
        },
        decision_variables: vec![],
        state_mutations: vec![],
        transient_data: vec![],
    });
    let input = CheckSolution {
        solution,
        contracts: vec![contract],
    };
    let response = client
        .post(url.join("/check-solution-with-contracts").unwrap())
        .json(&input)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200, "{}", response.text().await.unwrap());
    let value = response
        .json::<Option<CheckSolutionOutput>>()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        value,
        CheckSolutionOutput {
            utility: 1.0,
            gas: 0
        }
    );

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}

#[tokio::test]
async fn test_subscribe_blocks() {
    let solutions: Vec<_> = (0..200)
        .map(|i| solution_with_all_inputs_fixed_size(i, 4))
        .collect();
    let hashes: Vec<_> = solutions.iter().map(essential_hash::hash).collect();

    let mem = MemoryStorage::new();
    for solution in &solutions {
        mem.insert_solution_into_pool(solution.clone())
            .await
            .unwrap();
    }
    mem.move_solutions_to_solved(&hashes[0..1]).await.unwrap();

    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup_with_mem(mem.clone()).await;

    let a = url.join("/subscribe-blocks").unwrap();
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let mut s = make_stream(response);

    let block = s.try_next().await.unwrap().unwrap();
    assert_eq!(block.number, 0);
    assert_eq!(block.solutions.len(), 1);
    assert_eq!(essential_hash::hash(&block.solutions[0]), hashes[0]);

    let r = tokio::time::timeout(Duration::from_millis(50), s.try_next()).await;
    assert!(r.is_err());

    mem.move_solutions_to_solved(&hashes[1..3]).await.unwrap();

    let block = s.try_next().await.unwrap().unwrap();
    assert_eq!(block.number, 1);
    assert_eq!(block.solutions.len(), 2);
    assert_eq!(essential_hash::hash(&block.solutions[0]), hashes[1]);
    assert_eq!(essential_hash::hash(&block.solutions[1]), hashes[2]);
    drop(s);

    let mut a = url.join("/subscribe-blocks").unwrap();
    a.query_pairs_mut().append_pair("page", "0");

    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let result: Vec<_> = make_stream(response)
        .take_until(tokio::time::sleep(Duration::from_millis(50)))
        .try_collect()
        .await
        .unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].number, 0);
    assert_eq!(result[1].number, 1);

    let mut a = url.join("/subscribe-blocks").unwrap();
    a.query_pairs_mut().append_pair("page", "1");

    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let mut s = make_stream(response);

    let r = tokio::time::timeout(Duration::from_millis(50), s.try_next()).await;
    assert!(r.is_err());

    for hash in &hashes[3..] {
        mem.move_solutions_to_solved(&[*hash]).await.unwrap();
    }

    let block = s.try_next().await.unwrap().unwrap();
    assert_eq!(block.number, 100);

    let blocks: Vec<_> = s
        .take_until(tokio::time::sleep(Duration::from_millis(50)))
        .try_collect()
        .await
        .unwrap();
    assert_eq!(blocks.len(), 98);
    assert_eq!(blocks[97].number, 198);

    let mut a = url.join("/subscribe-blocks").unwrap();
    a.query_pairs_mut().append_pair("block", "190");

    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let s = make_stream(response);

    let blocks: Vec<_> = s
        .take_until(tokio::time::sleep(Duration::from_millis(50)))
        .try_collect()
        .await
        .unwrap();
    assert_eq!(blocks.len(), 9);
    assert_eq!(blocks[8].number, 198);

    let mut a = url.join("/subscribe-blocks").unwrap();
    a.query_pairs_mut()
        .append_pair("block", "90")
        .append_pair("page", "1");

    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);

    let s = make_stream(response);

    let blocks: Vec<_> = s
        .take_until(tokio::time::sleep(Duration::from_millis(50)))
        .try_collect()
        .await
        .unwrap();
    assert_eq!(blocks.len(), 9);
    assert_eq!(blocks[8].number, 198);

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}

struct BlockDecoder {}

impl Decoder for BlockDecoder {
    type Item = Block;
    type Error = anyhow::Error;

    fn decode(&mut self, buf: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let end = buf
            .iter()
            .zip(buf.iter().skip(1))
            .position(|(&a, &b)| a == b'\n' && b == b'\n');

        match end {
            Some(end) => {
                let s = std::str::from_utf8(&buf[..end])?;
                let s = s.trim_start_matches("data: ").trim();
                let block = serde_json::from_str::<Block>(s)?;
                buf.advance(end + 2);
                Ok(Some(block))
            }
            None => Ok(None),
        }
    }
}

fn make_stream(response: reqwest::Response) -> impl futures::Stream<Item = anyhow::Result<Block>> {
    let stream = StreamReader::new(
        response
            .bytes_stream()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{}", e))),
    );
    FramedRead::new(stream, BlockDecoder {})
}

fn contract_with_salt(salt: Word) -> Contract {
    let mut s = [0; 32];
    s[0..8].copy_from_slice(&bytes_from_word(salt));
    Contract {
        predicates: vec![],
        salt: s,
    }
}

#[tokio::test]
async fn test_subscribe_contracts() {
    let contracts: Vec<_> = (0..200)
        .map(contract_with_salt)
        .map(|mut c| {
            c.predicates.push(Predicate::empty());
            c
        })
        .map(sign_contract_with_random_keypair)
        .collect();

    let mem = MemoryStorage::new();
    mem.insert_contract(contracts[0].clone()).await.unwrap();

    let TestServer {
        client,
        url,
        shutdown,
        jh,
    } = setup_with_mem(mem.clone()).await;

    let a = url.join("/subscribe-contracts").unwrap();
    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let mut s = make_contract_stream(response);

    let contract = s.try_next().await.unwrap().unwrap();
    assert_eq!(contract, contracts[0].contract);

    let r = tokio::time::timeout(Duration::from_millis(50), s.try_next()).await;
    assert!(r.is_err());

    for contract in &contracts[1..3] {
        mem.insert_contract(contract.clone()).await.unwrap();
    }

    let result: Vec<_> = s
        .take_until(tokio::time::sleep(Duration::from_millis(50)))
        .try_collect()
        .await
        .unwrap();

    for (contract, result) in contracts[1..3].iter().zip(result) {
        assert_eq!(contract.contract, result);
    }

    let mut a = url.join("/subscribe-contracts").unwrap();
    a.query_pairs_mut().append_pair("page", "0");

    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let result: Vec<_> = make_contract_stream(response)
        .take_until(tokio::time::sleep(Duration::from_millis(50)))
        .try_collect()
        .await
        .unwrap();

    assert_eq!(result.len(), 3);
    for (contract, result) in contracts[0..3].iter().zip(result) {
        assert_eq!(contract.contract, result);
    }

    let mut a = url.join("/subscribe-contracts").unwrap();
    a.query_pairs_mut().append_pair("page", "1");

    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let mut s = make_contract_stream(response);

    let r = tokio::time::timeout(Duration::from_millis(50), s.try_next()).await;
    assert!(r.is_err());

    for contract in &contracts[3..] {
        mem.insert_contract(contract.clone()).await.unwrap();
    }

    let contract = s.try_next().await.unwrap().unwrap();
    assert_eq!(contract, contracts[100].contract);

    let results: Vec<_> = s
        .take_until(tokio::time::sleep(Duration::from_millis(50)))
        .try_collect()
        .await
        .unwrap();
    assert_eq!(results.len(), 99);
    assert_eq!(results[98], contracts[199].contract);

    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let start = time - std::time::Duration::from_secs(100);
    let mut a = url.join("/subscribe-contracts").unwrap();
    a.query_pairs_mut()
        .append_pair("start", start.as_secs().to_string().as_str());

    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let s = make_contract_stream(response);

    let results: Vec<_> = s
        .take_until(tokio::time::sleep(Duration::from_millis(50)))
        .try_collect()
        .await
        .unwrap();
    assert_eq!(results.len(), 200);
    assert_eq!(results[199], contracts[199].contract);

    let mut a = url.join("/subscribe-contracts").unwrap();
    a.query_pairs_mut()
        .append_pair("start", start.as_secs().to_string().as_str())
        .append_pair("page", "1");

    let response = client.get(a).send().await.unwrap();
    assert_eq!(response.status(), 200);

    let s = make_contract_stream(response);

    let results: Vec<_> = s
        .take_until(tokio::time::sleep(Duration::from_millis(50)))
        .try_collect()
        .await
        .unwrap();
    assert_eq!(results.len(), 100);
    for (contract, result) in contracts[100..].iter().zip(results) {
        assert_eq!(contract.contract, result);
    }

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}

struct ContractDecoder {}

impl Decoder for ContractDecoder {
    type Item = Contract;
    type Error = anyhow::Error;

    fn decode(&mut self, buf: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let end = buf
            .iter()
            .zip(buf.iter().skip(1))
            .position(|(&a, &b)| a == b'\n' && b == b'\n');

        match end {
            Some(end) => {
                let s = std::str::from_utf8(&buf[..end])?;
                let s = s.trim_start_matches("data: ").trim();
                let contract = serde_json::from_str::<Contract>(s);
                buf.advance(end + 2);
                let Ok(contract) = contract else {
                    return Ok(None);
                };
                Ok(Some(contract))
            }
            None => Ok(None),
        }
    }
}

fn make_contract_stream(
    response: reqwest::Response,
) -> impl futures::Stream<Item = anyhow::Result<Contract>> {
    let stream = StreamReader::new(
        response
            .bytes_stream()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{}", e))),
    );
    FramedRead::new(stream, ContractDecoder {})
}
