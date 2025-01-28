use super::server::Command;
use crate::{ChatMsg, ReqHistoryBody};
use anyhow::Context;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tracing::instrument;
use tracked_cancellations::TrackedCancellationToken;
use ws_helpers::heartbeat::HeartbeatConfig;
use wykies_shared::{log_err_as_error, session::UserSessionInfo, websockets::WsConnId};

/// Handle and command sender for chat server.
///
/// Reduces boilerplate of setting up response channels in WebSocket handlers.
#[derive(Debug, Clone)]
pub struct ChatServerHandle {
    cmd_tx: mpsc::Sender<Command>,
    pub heartbeat_config: HeartbeatConfig,
}
impl ChatServerHandle {
    pub(crate) fn new(
        cmd_tx: mpsc::Sender<super::server::Command>,
        heartbeat_config: HeartbeatConfig,
    ) -> Self {
        Self {
            cmd_tx,
            heartbeat_config,
        }
    }
}

impl ChatServerHandle {
    /// Register client message sender and obtain connection ID.
    #[instrument(skip())]
    pub async fn register(
        &self,
        conn_tx: mpsc::Sender<Arc<ChatMsg>>,
        user_info: UserSessionInfo,
    ) -> (WsConnId, TrackedCancellationToken) {
        let (res_tx, res_rx) = oneshot::channel();

        self.send_cmd_to_server(
            Command::Connect {
                conn_tx,
                user_info,
                res_tx,
            },
            res_rx,
        )
        .await
        .expect("failed to send command")
    }

    /// Broadcast message to other users
    #[instrument]
    pub async fn send_msg_to_clients(&self, msg: ChatMsg) {
        let (res_tx, res_rx) = oneshot::channel();

        self.send_cmd_to_server(Command::ForClients { msg, res_tx }, res_rx)
            .await
            .expect("failed to send command");
    }

    #[instrument]
    pub async fn process_history_request(&self, conn_id: &WsConnId, req: ReqHistoryBody) {
        let (res_tx, res_rx) = oneshot::channel();

        self.send_cmd_to_server(
            Command::HistoryReq {
                req,
                conn_id: conn_id.to_owned(),
                res_tx,
            },
            res_rx,
        )
        .await
        .expect("failed to send command");
    }

    #[instrument(skip(res_rx))]
    async fn send_cmd_to_server<T>(
        &self,
        cmd: Command,
        res_rx: oneshot::Receiver<T>,
    ) -> anyhow::Result<T> {
        self.cmd_tx
            .send(cmd)
            .await
            .context("sending command to chat server failed")?;

        res_rx
            .await
            .context("failed to get a response from the chat server")
    }

    /// Unregister message sender and broadcast disconnection message
    #[instrument(skip())]
    pub async fn unregister(&self, conn: WsConnId) {
        let r = self
            .cmd_tx
            .send(Command::Disconnect { conn })
            .await
            .context("chat server should not have been dropped");
        log_err_as_error!(r);
    }
}
