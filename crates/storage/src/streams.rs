use std::time::Duration;

#[cfg(test)]
mod tests;

/// Notify that there are new contracts or blocks.
#[derive(Clone)]
pub struct Notify {
    contracts: tokio::sync::watch::Sender<()>,
    blocks: tokio::sync::watch::Sender<()>,
}

/// Wait for new data.
#[derive(Clone)]
pub struct NewData(tokio::sync::watch::Receiver<()>);

/// State of the stream.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StreamState {
    state: State,
    start: Start,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum State {
    Pos(Pos),
    Done,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Pos {
    page: usize,
    index: usize,
}

/// Get data from this point.
#[derive(Default, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GetData {
    /// Page number to get data from.
    pub page: usize,
    /// Time to get data from.
    pub time: Option<Duration>,
    /// Number to get data from.
    pub number: Option<u64>,
}

#[derive(Default, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Start {
    time: Option<Duration>,
    number: Option<u64>,
}

/// Get the next data in the stream.
pub async fn next_data<F, Fut, D>(
    mut new_data: NewData,
    state: StreamState,
    page_size: usize,
    get_data: F,
) -> Option<(Vec<anyhow::Result<D>>, StreamState)>
where
    F: Fn(GetData) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<Vec<D>>>,
{
    // Check if the stream is done due to returning an error.
    let pos = match state.state {
        State::Pos(pos) => pos,
        State::Done => return None,
    };

    // Loop while there are no new data to return but there has been a change in contracts.
    loop {
        // List the data for this page
        let data = get_data(GetData {
            page: pos.page,
            time: state.start.time,
            number: state.start.number,
        })
        .await;

        match data {
            // If there are no data for this page, await a change.
            Ok(data) if data.get(pos.index..).filter(|d| !d.is_empty()).is_none() => {
                match new_data.wait().await {
                    // Got a change, get data again.
                    Ok(_) => continue,
                    // The new data channel was closed, this means
                    // the program is shutting down. Close this stream.
                    Err(_) => return None,
                }
            }
            // There is some data to return.
            Ok(mut data) => {
                // Calculate the next page and index.
                let next_page = if data.len() >= page_size {
                    Pos {
                        page: pos.page + 1,
                        index: 0,
                    }
                } else {
                    Pos {
                        page: pos.page,
                        index: data.len(),
                    }
                };

                // Drain just the new data (this should never be empty due to the above check).
                return Some((
                    data.drain(pos.index..).map(Ok).collect::<Vec<_>>(),
                    StreamState {
                        state: State::Pos(next_page),
                        start: state.start,
                    },
                ));
            }
            // Got an error so return the error and mark the stream as done.
            Err(e) => {
                return Some((
                    vec![Err(e)],
                    StreamState {
                        state: State::Done,
                        start: state.start,
                    },
                ))
            }
        }
    }
}

impl StreamState {
    /// Create a new stream state from a page.
    pub fn new(page: Option<usize>, time: Option<Duration>, number: Option<u64>) -> Self {
        let page = page.unwrap_or(0);
        Self {
            state: State::Pos(Pos { page, index: 0 }),
            start: Start { time, number },
        }
    }
}

impl Notify {
    /// Create a new notify.
    pub fn new() -> Self {
        let (contracts, _) = tokio::sync::watch::channel(());
        let (blocks, _) = tokio::sync::watch::channel(());
        Self { contracts, blocks }
    }

    /// Notify that there are new contracts.
    pub fn notify_new_contracts(&self) {
        // There might not be any subscribers so we
        // need to ignore the error.
        let _ = self.contracts.send(());
    }

    /// Notify that there are new blocks.
    pub fn notify_new_blocks(&self) {
        // There might not be any subscribers so we
        // need to ignore the error.
        let _ = self.blocks.send(());
    }

    /// Subscribe to new contracts.
    pub fn subscribe_contracts(&self) -> NewData {
        NewData(self.contracts.subscribe())
    }

    /// Subscribe to new blocks.
    pub fn subscribe_blocks(&self) -> NewData {
        NewData(self.blocks.subscribe())
    }
}

impl NewData {
    /// Wait for new data.
    /// Returns an error if the channel is closed.
    pub async fn wait(&mut self) -> anyhow::Result<()> {
        self.0
            .changed()
            .await
            .map_err(|_| anyhow::anyhow!("channel closed"))
    }
}

impl Default for Notify {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for StreamState {
    fn default() -> Self {
        StreamState {
            state: State::Pos(Pos { page: 0, index: 0 }),
            start: Start::default(),
        }
    }
}
