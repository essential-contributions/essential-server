//! CLI tool for dry running a solution check on Essential server.

use clap::Parser;
use essential_dry_run::{dry_run_solution, read_intent_sets};
use essential_types::solution::Solution;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Path to compiled intents.
    #[arg(short, long)]
    intents: PathBuf,
    /// Solution to check in JSON string format.
    #[arg(short, long)]
    solution: String,
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    if let Err(e) = run(args).await {
        eprintln!("Command failed because: {}", e);
    }
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    let intents = read_intent_sets(&cli.intents)
        .await?
        .into_iter()
        .flatten()
        .collect();
    let solution: Solution = serde_json::from_str(&cli.solution)?;
    let output = dry_run_solution(intents, solution).await?;
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
