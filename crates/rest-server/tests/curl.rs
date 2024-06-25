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
