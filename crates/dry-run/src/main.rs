use clap::{Parser, Subcommand};
use essential_memory_storage::MemoryStorage;
use essential_types::{intent::Intent, solution::Solution};
use std::path::{Path, PathBuf};
use tokio::{
    io::{AsyncReadExt, BufReader},
    task::JoinHandle,
};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Select a subcommand to run
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    CreateAccount {
        /// Set the path to the wallet directory.
        /// If not set then a sensible default will be used (like ~/.essential-wallet).
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// The name of the account to create.
        account: String,
    },
    /// Dry run a solution after signing and deploying intents to local memory storage.
    DeployAndCheck {
        /// Set the path to the wallet directory.
        /// If not set then a sensible default will be used (like ~/.essential-wallet).
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// The name of the account to deploy the app with.
        #[arg(long)]
        account: String,
        /// Path to compiled intents.
        #[arg(long)]
        intents: PathBuf,
        /// Solution to check in JSON.
        #[arg(long)]
        solution: String,
    },
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    if let Err(e) = run(args).await {
        eprintln!("Command failed because: {}", e);
    }
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::CreateAccount { account, path } => {
            let mut wallet = get_wallet(path)?;
            wallet.new_key_pair(&account, essential_wallet::Scheme::Secp256k1)?;
            println!("Created account: {}", account);
        }
        Command::DeployAndCheck {
            path,
            account,
            intents,
            solution,
        } => {
            let mut wallet = get_wallet(path)?;

            let jh: JoinHandle<anyhow::Result<()>> = tokio::task::spawn(async move {
                let storage = MemoryStorage::new();
                let essential = essential_server::Essential::new(storage, Default::default());
                let intents =
                    sign_and_deploy_intents(&intents, &mut wallet, &account, &essential).await?;
                let solution: Solution = serde_json::from_str(&solution)?;

                let output = essential
                    .check_solution_with_data(solution, intents)
                    .await?;

                println!("{}", serde_json::to_string(&output)?);
                Ok(())
            });

            jh.await
                .expect("Server dry run failed")
                .expect("Server dry run error");
        }
    }
    Ok(())
}

// TODO: duplicate in `essential-deploy-intent`
fn get_wallet(path: Option<PathBuf>) -> anyhow::Result<essential_wallet::Wallet> {
    let pass = rpassword::prompt_password("Enter password to unlock wallet: ")?;
    let wallet = match path {
        Some(path) => essential_wallet::Wallet::new(&pass, path)?,
        None => essential_wallet::Wallet::with_default_path(&pass)?,
    };
    Ok(wallet)
}

async fn sign_and_deploy_intents(
    intents_path: &Path,
    wallet: &mut essential_wallet::Wallet,
    account: &str,
    essential: &essential_server::Essential<MemoryStorage>,
) -> anyhow::Result<Vec<Intent>> {
    let mut intents: Vec<Intent> = vec![];
    for intent in intents_path.read_dir()? {
        let name = intent?.file_name();
        let name = name
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("invalid file name"))?;
        let path = intents_path.join(name);

        let intent_set = read_intents(&path).await?;
        let signed_set = wallet.sign_intent_set(intent_set.clone(), account)?;
        essential.deploy_intent_set(signed_set).await?;

        intents.extend(intent_set);
    }
    Ok(intents)
}

async fn read_intents(path: &Path) -> anyhow::Result<Vec<Intent>> {
    let file = tokio::fs::File::open(path).await?;
    let mut bytes = Vec::new();
    let mut reader = BufReader::new(file);
    reader.read_to_end(&mut bytes).await?;
    Ok(serde_json::from_slice::<Vec<Intent>>(&bytes)?)
}
