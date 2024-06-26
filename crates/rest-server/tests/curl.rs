use std::fmt::Display;

use essential_server_types::{CheckSolution, QueryStateReads};
use essential_types::{intent::Intent, IntentAddress};
use test_utils::{empty::Empty, sign_intent_set_with_random_keypair, solution_with_intent};
use tokio::process::Command;
use utils::{setup, TestServer};

mod utils;

#[tokio::test]
async fn test_readme_curl() {
    let readme = tokio::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))
        .await
        .unwrap();

    let TestServer {
        url, shutdown, jh, ..
    } = setup().await;

    let port = url.port().unwrap();

    let commands: Vec<_> = readme
        .split("```bash")
        .skip(1)
        .filter_map(|s| s.split("```").next())
        .filter(|s| s.trim().starts_with("curl"))
        .map(|s| s.trim())
        .map(|s| {
            let mut s = s.to_string();
            s = s.replace(
                "Content-Type: application/json",
                "Content-Type:application/json",
            );
            s = s.replace('\'', "\"");

            s.split(' ')
                .map(|s| s.trim_start_matches('"').trim_end_matches('"'))
                .map(|s| s.trim())
                .filter(|s| !s.is_empty() && *s != "\\")
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        })
        .map(|mut s| {
            if let Some(a) = s.iter_mut().find(|s| s.starts_with("http://localhost:")) {
                if let Some(p) = a.split(':').nth(2).and_then(|s| s.split('/').next()) {
                    *a = a.replace(p, &port.to_string());
                }
            }
            s
        })
        .collect();

    for c in &commands {
        let mut command = Command::new("curl");
        for arg in c.iter().skip(1) {
            command.arg(arg);
        }
        let output = command.output().await.unwrap();
        let s = String::from_utf8_lossy(&output.stdout);
        assert!(!s.contains("failed") && !s.contains("Failed"));
        assert!(output.status.success());
    }

    shutdown.send(()).unwrap();
    jh.await.unwrap().unwrap();
}

#[test]
#[ignore = "Just a utility, not really a test"]
fn create_readme_inputs() {
    let predicate = Intent::empty();
    let signed = sign_intent_set_with_random_keypair(vec![predicate]);
    let address = IntentAddress {
        set: essential_hash::intent_set_addr::from_intents(&signed.set),
        intent: essential_hash::content_addr(&signed.set[0]),
    };
    let solution = solution_with_intent(address.clone());
    fn ser<T: serde::Serialize>(t: T) {
        println!("{}", serde_json::to_string(&t).unwrap());
    }
    fn p(s: impl Display) {
        println!("{}", s);
    }
    p("deploy contract");
    ser(&signed);

    p("get contract");
    p(&address.set);

    p("get predicate");
    p(&address.set);
    p(&address.intent);

    p("submit solution");
    ser(&solution);

    p("query state");
    p(&address.set);
    p(hex::encode_upper(vec![0]));

    p("solution outcome");
    p(essential_hash::content_addr(&solution));

    p("check solution");
    ser(&solution);

    p("check solution with data");
    ser(CheckSolution {
        solution: solution.clone(),
        intents: signed.set.clone(),
    });

    p("query-state-reads");
    let query =
        QueryStateReads::from_solution(solution.clone(), 0, &signed.set[0], Default::default());
    ser(query);
}
