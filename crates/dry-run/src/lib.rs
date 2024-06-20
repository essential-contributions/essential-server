//! Dry run of checking a solution on Essential server.
//!
//! This crate can be used as a library and a binary CLI tool.

#![deny(missing_docs)]
#![deny(unsafe_code)]

use essential_memory_storage::MemoryStorage;
use essential_server::CheckSolutionOutput;
use essential_types::{intent::Intent, solution::Solution};
use std::path::Path;
use tokio::io::{AsyncReadExt, BufReader};

/// Dry run a solution check with given intents.
pub async fn dry_run_solution(
    intents: Vec<Intent>,
    solution: Solution,
) -> anyhow::Result<CheckSolutionOutput> {
    let storage = MemoryStorage::new();
    let essential = essential_server::Essential::new(storage, Default::default());
    let output = essential
        .check_solution_with_data(solution, intents)
        .await?;
    Ok(output)
}

/// Read intent sets in a directory.
pub async fn read_intent_sets(path: &Path) -> anyhow::Result<Vec<Vec<Intent>>> {
    let mut intents: Vec<Vec<Intent>> = vec![];
    for intent in path.read_dir()? {
        let name = intent?.file_name();
        let name = name
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("invalid file name"))?;
        let path = path.join(name);
        let intent_set = read_intents(&path).await?;
        intents.push(intent_set);
    }
    Ok(intents)
}

/// Read intents from a file.
pub async fn read_intents(path: &Path) -> anyhow::Result<Vec<Intent>> {
    let file = tokio::fs::File::open(path).await?;
    let mut bytes = Vec::new();
    let mut reader = BufReader::new(file);
    reader.read_to_end(&mut bytes).await?;
    Ok(serde_json::from_slice::<Vec<Intent>>(&bytes)?)
}
