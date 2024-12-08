use std::time::Duration;

use tokio::sync::mpsc;
use tokio_util::sync::{CancellationToken, DropGuard, WaitForCancellationFuture};
use tracing::{info, instrument, warn};
use wykies_time::Seconds;

#[derive(Debug, Clone)]
pub struct TrackedCancellationToken {
    token: CancellationToken,
    /// When all tasks complete they will drop the senders
    #[allow(unused)]
    drop_tracker: mpsc::Sender<()>,
}

#[derive(Debug)]
/// Allows checking  if all the `TrackedCancellationTokens` have been dropped
pub struct CancellationTracker {
    token: CancellationToken,
    /// Can be awaited to see when all Tracked Tokens have been dropped
    rx: mpsc::Receiver<()>,
}

impl TrackedCancellationToken {
    #[instrument]
    pub fn new() -> (Self, CancellationTracker) {
        let (drop_tracker, rx) = mpsc::channel(1);
        let token = CancellationToken::new();
        (
            Self {
                token: token.clone(),
                drop_tracker,
            },
            CancellationTracker { token, rx },
        )
    }

    #[instrument]
    pub fn cancel(&self) {
        self.token.cancel()
    }

    pub fn cancelled(&self) -> WaitForCancellationFuture {
        self.token.cancelled()
    }

    pub fn drop_guard(self) -> DropGuard {
        self.token.drop_guard()
    }
}

impl CancellationTracker {
    #[instrument]
    pub fn cancel(&self) {
        self.token.cancel()
    }

    #[instrument]
    pub async fn await_cancellations(&mut self, timeout: Duration) {
        match tokio::time::timeout(timeout, self.rx.recv()).await {
            Ok(_) => info!("All tracked cancellation tokens have been dropped"),
            Err(_elapsed) => warn!(
                "Timed out waiting for tracking tokens to be dropped after {:?}",
                Seconds::from(timeout)
            ),
        }
    }
}
