// TODO: Remove this
#![allow(dead_code)]
#![allow(unused_variables)]

use tokio::sync::oneshot;

pub struct Handle {
    tx: oneshot::Sender<()>,
    jh: Option<tokio::task::JoinHandle<anyhow::Result<()>>>,
}

pub struct Shutdown(oneshot::Receiver<()>);

pub async fn run<S>(storage: &S, shutdown: Shutdown) -> anyhow::Result<()> {
    // Check for solutions in pool that solve intents.
    // Search for best batch of solutions.
    // Move solved solutions out of pool and into solved db.
    shutdown.0.await?;
    Ok(())
}

impl Handle {
    pub fn new() -> (Self, Shutdown) {
        let (tx, rx) = oneshot::channel();
        (Self { tx, jh: None }, Shutdown(rx))
    }

    pub fn set_jh(&mut self, jh: tokio::task::JoinHandle<anyhow::Result<()>>) {
        self.jh = Some(jh);
    }

    pub async fn shutdown(self) -> anyhow::Result<()> {
        self.tx
            .send(())
            .map_err(|_| anyhow::anyhow!("Failed to send shutdown signal"))?;
        if let Some(jh) = self.jh {
            jh.await??;
        }
        Ok(())
    }
}
