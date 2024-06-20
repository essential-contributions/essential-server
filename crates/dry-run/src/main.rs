//! CLI tool for dry running a solution check on Essential server.

use clap::Parser;
use essential_dry_run::dry_run_from_path;
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
    let output = dry_run_from_path(&cli.intents, cli.solution).await?;
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
