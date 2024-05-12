use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Address to bind to.
    #[arg(default_value_t = String::from("0.0.0.0:0"))]
    address: String,

    /// Type of database to use.
    #[arg(long, short, default_value_t = Db::Memory, value_enum)]
    db: Db,

    #[arg(long, short, default_value_t = String::from("https://localhost:4001"))]
    /// Address of the rqlite server, if using rqlite.
    rqlite_address: String,
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
        rqlite_address,
    } = Cli::parse();
    let (local_addr, local_addr_rx) = tokio::sync::oneshot::channel();
    let config = Default::default();
    let jh = tokio::task::spawn(async move {
        match db {
            Db::Memory => {
                let storage = memory_storage::MemoryStorage::new();
                let essential = essential_server::Essential::new(storage, config);
                essential_rest_server::run(essential, address, local_addr, None).await
            }
            Db::Rqlite => {
                let storage = rqlite_storage::RqliteStorage::new(&rqlite_address)
                    .await
                    .expect("Failed to connect to rqlite");
                let essential = essential_server::Essential::new(storage, config);
                essential_rest_server::run(essential, address, local_addr, None).await
            }
        }
    });
    let local_addr = local_addr_rx.await.expect("Failed to get local address");
    println!("Listening on: {}", local_addr);
    jh.await.expect("Server failed").expect("Server Error");
}
