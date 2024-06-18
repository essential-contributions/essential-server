use std::time::Duration;

use clap::{Parser, ValueEnum};
use essential_memory_storage::MemoryStorage;
use essential_rest_server::Config;
use essential_rqlite_storage::RqliteStorage;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Address to bind to.
    #[arg(default_value_t = String::from("0.0.0.0:0"))]
    address: String,

    /// Type of database to use.
    #[arg(long, short, default_value_t = Db::Memory, value_enum)]
    db: Db,

    /// Disable block building and only handle queries.
    #[arg(long)]
    disable_block_building: bool,

    #[arg(long, short, default_value_t = String::from("https://localhost:4001"))]
    /// Address of the rqlite server, if using rqlite.
    rqlite_address: String,

    #[arg(long)]
    /// Enable tracing.
    disable_tracing: bool,

    #[arg(long, short)]
    /// Frequency at which to run the main loop in seconds.
    loop_freq: Option<u64>,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum Db {
    Memory,
    Rqlite,
}

#[tokio::main]
async fn main() {
    let Cli {
        address,
        db,
        disable_block_building,
        rqlite_address,
        disable_tracing,
        loop_freq,
    } = Cli::parse();
    let (local_addr, local_addr_rx) = tokio::sync::oneshot::channel();
    let check_config = Default::default();
    if !disable_tracing {
        #[cfg(feature = "tracing")]
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::builder()
                    .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                    .from_env_lossy(),
            )
            .try_init();
    }

    let mut config = Config {
        build_blocks: !disable_block_building,
        ..Default::default()
    };
    if let Some(run_loop_interval) = loop_freq {
        config.server_config.run_loop_interval = Duration::from_secs(run_loop_interval);
    }

    let jh = tokio::task::spawn(async move {
        match db {
            Db::Memory => {
                let storage = MemoryStorage::new();
                let essential = essential_server::Essential::new(storage, check_config);
                essential_rest_server::run(essential, address, local_addr, None, config).await
            }
            Db::Rqlite => {
                let storage = RqliteStorage::new(&rqlite_address)
                    .await
                    .expect("Failed to connect to rqlite");
                let essential = essential_server::Essential::new(storage, check_config);
                essential_rest_server::run(essential, address, local_addr, None, config).await
            }
        }
    });
    let local_addr = local_addr_rx.await.expect("Failed to get local address");
    println!("Listening on: {}", local_addr);
    jh.await.expect("Server failed").expect("Server Error");
}
