use std::process::Stdio;

use essential_rqlite_storage::RqliteStorage;
use tempfile::TempDir;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
};

const DB: &str = "http://127.0.0.1:";
const URL: &str = "127.0.0.1:0";
// Rqlite complains if we use the same url for both http and raft
const RAFT_URL: &str = "localhost:0";
const PORT_LINE: &str = "service listening on";
const LEADER: &str = "is now Leader";

pub struct TestRqlite {
    pub temp_dir: TempDir,
    pub command: Child,
    pub url: String,
    pub rqlite: RqliteStorage,
}

impl TestRqlite {
    pub async fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();

        let mut child = Command::new("rqlited")
            .arg("-node-id")
            .arg("1")
            .arg("-http-addr")
            .arg(URL)
            .arg("-raft-addr")
            .arg(RAFT_URL)
            .arg("-fk")
            .arg(&format!("{}", temp_dir.path().display()))
            .kill_on_drop(true)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let stderr = child.stderr.take().unwrap();

        let buf = BufReader::new(stderr);
        let mut lines = buf.lines();

        let mut port = 0;
        loop {
            if let Some(line) = lines.next_line().await.unwrap() {
                eprintln!("{}", line);
                if line.contains("[http]") && line.contains(PORT_LINE) {
                    port = line
                        .split(PORT_LINE)
                        .nth(1)
                        .unwrap()
                        .split(':')
                        .next_back()
                        .unwrap()
                        .trim()
                        .parse::<u16>()
                        .unwrap();
                }
                if line.contains(LEADER) {
                    break;
                }
            }
        }

        child.stderr = Some(lines.into_inner().into_inner());

        assert_ne!(port, 0);
        let url = format!("{}{}", DB, port);

        let rqlite = RqliteStorage::new(&url).await.unwrap();

        Self {
            temp_dir,
            command: child,
            url,
            rqlite,
        }
    }
}
