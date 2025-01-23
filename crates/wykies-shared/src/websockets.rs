use crate::token::AuthToken;
use anyhow::{bail, Context as _};
use ewebsock::{WsEvent, WsMessage};
use std::{
    fmt::{Debug, Display},
    time::{Duration, Instant},
};
use tracing::{instrument, warn};
use uuid::Uuid;
use wykies_time::Seconds;

#[derive(
    Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct WSConnId(Uuid);

pub struct WSConnTxRx {
    tx: ewebsock::WsSender,
    rx: ewebsock::WsReceiver,
}

pub struct WSConnWithID {
    pub id: WSConnId,
    pub conn: WSConnTxRx,
}

impl WSConnId {
    pub fn new_rand() -> Self {
        Self(Uuid::new_v4())
    }
}

pub trait WakeFn: Fn() + Send + Sync + 'static + Clone {}
impl<T> WakeFn for T where T: Fn() + Send + Sync + 'static + Clone {}

#[inline]
pub fn wake_fn(ctx: egui::Context) -> impl WakeFn {
    move || ctx.request_repaint()
}

impl Display for WSConnId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Debug for WSConnTxRx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebSocketConnection {{ tx, rx }} ")
    }
}

impl WSConnTxRx {
    #[instrument(skip(wake_up))]
    pub fn initiate_connection<F, S>(ws_url: S, wake_up: F) -> anyhow::Result<WSConnTxRx>
    where
        F: WakeFn,
        S: Into<String> + Debug,
    {
        let (tx, rx) = ewebsock::connect_with_wakeup(ws_url, Default::default(), wake_up)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("failed to connect web socket")?;
        Ok(WSConnTxRx { tx, rx })
    }

    #[inline]
    pub fn send(&mut self, msg: WsMessage) {
        self.tx.send(msg);
    }

    /// Try receiving a new event without blocking.
    #[inline]
    pub fn try_recv(&self) -> Option<WsEvent> {
        self.rx.try_recv()
    }

    #[instrument]
    pub fn close(mut self) {
        self.tx.close();
    }

    /// Provides a cancellation safe way to wait until a message is received
    pub async fn recv(&mut self) -> WsEvent {
        // TODO 4: Review uses of recv and add timeout
        loop {
            if let Some(m) = self.try_recv() {
                break m;
            } else {
                reqwest_cross::yield_now().await;
            }
        }
    }

    pub async fn recv_with_timeout_ignoring_ping(
        &self,
        timeout: Seconds,
    ) -> anyhow::Result<ewebsock::WsEvent> {
        let start = Instant::now();
        let timeout_duration: Duration = timeout.into();
        while start.elapsed() < timeout_duration {
            if let Some(msg) = self.try_recv() {
                if matches!(
                    &msg,
                    ewebsock::WsEvent::Message(ewebsock::WsMessage::Ping(_))
                ) {
                    continue; // Skip ping messages
                }
                return Ok(msg);
            } else {
                reqwest_cross::yield_now().await
            }
        }
        bail!("Receiving timed out after {timeout:?} seconds")
    }

    #[instrument(skip(wake_up))]
    pub async fn initiate_connection_with_auth<F, S>(
        token: AuthToken,
        ws_url: S,
        wake_up: F,
    ) -> anyhow::Result<WSConnTxRx>
    where
        F: WakeFn,
        S: Into<String> + Debug,
    {
        // Initiate connection
        let mut result = WSConnTxRx::initiate_connection(ws_url, wake_up)?;

        // Wait for connection to open before sending token
        result.wait_for_connection_to_open().await?;

        // Send token
        result.send(token.into());

        Ok(result)
    }

    #[tracing::instrument(ret, err(Debug))]
    pub async fn wait_for_connection_to_open(&mut self) -> anyhow::Result<()> {
        let event = self.recv().await;

        let base_err_msg = "expected first websocket event to be opened but instead got a";

        match event {
            WsEvent::Opened => Ok(()),
            WsEvent::Message(ws_message) => {
                bail!("{base_err_msg} message: {ws_message:?}")
            }
            WsEvent::Error(err_msg) => {
                // Using I'm A Tea Pot as unable to send more detailed error back
                if err_msg.contains("418") {
                    // Using I'm a teapot to communicate it's an Unexpected Client as we can only
                    // get the status code
                    warn!("UnexpectedClient");
                    bail!("Server Reported Unexpected Connection (This may happen sometimes but should not happen very often)")
                } else {
                    bail!("{base_err_msg}n error: {err_msg}")
                }
            }
            WsEvent::Closed => {
                bail!("{base_err_msg} Closed event")
            }
        }
    }
}
