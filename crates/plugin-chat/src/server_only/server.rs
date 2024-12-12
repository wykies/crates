use crate::{
    consts::{CHAT_HISTORY_RECENT_CAPACITY, CHAT_MAX_IMS_BEFORE_SAVE, CHAT_MAX_TIME_BEFORE_SAVE},
    ChatIM, ChatMsg, ChatUser, InitialStateBody, ReqHistoryBody, RespHistoryBody,
};
use anyhow::{anyhow, bail, Context};
use std::{collections::HashMap, fmt::Debug, sync::Arc};
use tokio::{
    select,
    sync::{mpsc, oneshot},
};
use tracing::{error, info, instrument, warn};
use tracked_cancellations::TrackedCancellationToken;
use ws_auth::{WsConnId, WsId};
use wykies_server::{
    db_types::DbPool, log_err_as_error, log_err_as_warn, ws::HeartbeatConfig, ServerTask,
    WebSocketSettings,
};
use wykies_shared::{const_config::CHANNEL_BUFFER_SIZE, debug_panic, session::UserSessionInfo};

use super::{history::ChatHistory, ChatSettings};

/// A command received by the [`ChatServer`].
#[derive(Debug)]
enum Command {
    Connect {
        conn_tx: mpsc::Sender<Arc<ChatMsg>>,
        user_info: Arc<UserSessionInfo>,
        res_tx: oneshot::Sender<(WsConnId, TrackedCancellationToken)>,
    },

    Disconnect {
        conn: WsConnId,
    },

    ForClients {
        msg: ChatMsg,
        skip: Option<WsConnId>,
        res_tx: oneshot::Sender<()>,
    },

    HistoryReq {
        req: ReqHistoryBody,
        conn_id: WsConnId,
        res_tx: oneshot::Sender<()>,
    },
}

#[derive(Debug)]
pub struct ChatServer {
    /// Map of connection IDs to their message receivers.
    connections: HashMap<WsConnId, mpsc::Sender<Arc<ChatMsg>>>,

    /// Command receiver.
    cmd_rx: mpsc::Receiver<Command>,

    history: ChatHistory,
    db_pool: DbPool,
}

/// Handle and command sender for chat server.
///
/// Reduces boilerplate of setting up response channels in WebSocket handlers.
#[derive(Debug, Clone)]
pub struct ChatServerHandle {
    cmd_tx: mpsc::Sender<Command>,
    ws_id: WsId,
    pub heartbeat_config: HeartbeatConfig,
}

impl ChatServer {
    pub fn new(
        config: &ChatSettings,
        ws_config: &WebSocketSettings,
        db_pool: DbPool,
        cancellation_token: TrackedCancellationToken,
    ) -> (Self, ChatServerHandle) {
        let (cmd_tx, cmd_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

        let heartbeat_config =
            HeartbeatConfig::new(config.heartbeat_interval_secs.into(), ws_config);

        let history = ChatHistory::new(
            CHAT_HISTORY_RECENT_CAPACITY,
            db_pool.clone(),
            cancellation_token,
            CHAT_MAX_TIME_BEFORE_SAVE,
            CHAT_MAX_IMS_BEFORE_SAVE,
        );

        (
            Self {
                connections: HashMap::new(),
                cmd_rx,
                history,
                db_pool,
            },
            ChatServerHandle {
                cmd_tx,
                heartbeat_config,
                ws_id: config.ws_id,
            },
        )
    }

    /// Send message to other users
    #[instrument]
    async fn send_msg_to_clients(
        &mut self,
        skip: Option<WsConnId>,
        msg: ChatMsg,
    ) -> anyhow::Result<()> {
        let msg = Arc::new(msg);

        // Save a copy of the IMs in recent history
        if let ChatMsg::IM(im) = msg.as_ref() {
            self.history
                .push(im.clone())
                .await
                .context("failed to add IM to history")?;
        }

        for (conn_id, tx) in self.connections.iter() {
            if skip.as_ref().is_some_and(|x| x == conn_id) {
                continue;
            }
            // errors if client disconnected abruptly and hasn't been timed-out yet
            let r = tx.send(Arc::clone(&msg)).await.with_context(|| {
                format!("failed to send message to one of the clients. Connection id {conn_id:?}")
            });
            log_err_as_warn!(r);
        }
        Ok(())
    }

    #[instrument]
    async fn send_history(&self, req: ReqHistoryBody, conn_id: WsConnId) {
        let outcome = sqlx::query!(
            "
SELECT `Author`,`Timestamp`,`Content`
FROM chat WHERE `Timestamp` <= ?
ORDER BY `Timestamp` DESC LIMIT ?",
            req.latest_timestamp.as_secs_since_unix_epoch(),
            req.qty
        )
        .fetch_all(&self.db_pool)
        .await
        .context("failed to get ims");

        let rows = match outcome {
            Ok(x) => x,
            Err(e) => {
                // Abort Error occurred
                log_err_as_error!(Err::<(), anyhow::Error>(e));
                return;
            }
        };

        let result = rows
            .into_iter()
            .map(|x| {
                Ok(ChatIM {
                    author: x.Author.try_into()?,
                    timestamp: x.Timestamp.into(),
                    content: x.Content.try_into()?,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()
            .context("failed to convert rows from DB into chat history");

        let mut result = match result {
            Ok(x) => x,
            Err(e) => {
                // Abort Error occurred
                log_err_as_error!(Err::<(), anyhow::Error>(e));
                return;
            }
        };

        // Reverse result was sorted the wrong way for LIMIT to get right messages
        result = result.into_iter().rev().collect();
        self.send_to_client(conn_id, ChatMsg::RespHistory(RespHistoryBody::new(result)))
            .await;
    }

    #[instrument]
    async fn send_to_client(&self, conn_id: WsConnId, msg: ChatMsg) {
        let msg = Arc::new(msg);

        let Some(tx) = self.connections.get(&conn_id) else {
            error!("failed to send message to client because unable to locate connection for ID: {conn_id:?}");
            debug_panic!("connection id not found");
            return;
        };

        let r = tx
            .send(Arc::clone(&msg))
            .await
            .with_context(|| format!("failed to send message to connection with id {conn_id:?}"));
        log_err_as_error!(r);
    }

    /// Register new connection and assign unique ID to this connection
    #[instrument(skip())]
    async fn register_connection(
        &mut self,
        tx: mpsc::Sender<Arc<ChatMsg>>,
        user_info: Arc<UserSessionInfo>,
    ) -> anyhow::Result<WsConnId> {
        // notify all users
        self.send_msg_to_clients(
            None,
            ChatMsg::UserJoined(ChatUser::new(user_info.username.clone())),
        )
        .await
        .context("failed to send msg to clients")?;

        // register session using a connection ID
        let id = WsConnId::new(user_info);
        self.connections.insert(id.clone(), tx.clone());

        // Send initial connection information
        self.send_initial_state(tx).await;

        // send id back
        Ok(id)
    }

    /// Returns the list of currently connected users with their multiplicity
    #[instrument]
    fn get_connected_users(&self) -> Vec<(ChatUser, u8)> {
        self.connections
            .keys()
            .map(|id| ChatUser::new(id.user_info.username.clone()))
            .fold(HashMap::<ChatUser, u8>::new(), |mut map, user| {
                let freq = map.entry(user).or_default();
                *freq = freq.saturating_add(1);
                map
            })
            .into_iter()
            .collect()
    }

    #[instrument]
    async fn send_initial_state(&self, tx: mpsc::Sender<Arc<ChatMsg>>) {
        let connected_users = self.get_connected_users();
        let history = RespHistoryBody::new(self.history.get_recent());
        let msg = Arc::new(ChatMsg::InitialState(InitialStateBody::new(
            connected_users,
            history,
        )));
        let r = tx
            .send(msg)
            .await
            .map_err(|msg| anyhow::anyhow!("failed to send initial state of: {msg:?}"));
        log_err_as_error!(r);
    }

    #[instrument]
    async fn unregister_connection(&mut self, conn_id: WsConnId) -> anyhow::Result<()> {
        // remove sender
        let remove_result = self.connections.remove(&conn_id);
        if remove_result.is_none() {
            warn!("Failed to remove connection with ID: {conn_id:?}");
        }
        // Notify other users of disconnect
        self.send_msg_to_clients(
            None,
            ChatMsg::UserLeft(ChatUser::new(conn_id.user_info.username.clone())),
        )
        .await
        .context("failed to unregister connection")
    }

    #[instrument(err(Debug))]
    async fn process_cmd(
        &mut self,
        cmd: Option<Command>,
        cancellation_token: TrackedCancellationToken,
    ) -> anyhow::Result<()> {
        let Some(cmd) = cmd else {
            bail!("None received by ChatServer on Command Channel. Shutting Down")
        };

        match cmd {
            Command::Connect {
                conn_tx,
                user_info,
                res_tx,
            } => {
                let conn_id = self
                    .register_connection(conn_tx, user_info)
                    .await
                    .context("failed to registering connection")?;
                self.send_response(res_tx, (conn_id, cancellation_token))
                    .await;
            }

            Command::Disconnect { conn } => {
                self.unregister_connection(conn)
                    .await
                    .context("fatal error to unregister a connection")?;
            }

            Command::ForClients { skip, msg, res_tx } => {
                self.send_msg_to_clients(skip, msg)
                    .await
                    .context("failed to sent IM to clients")?;
                self.send_response(res_tx, ()).await;
            }

            Command::HistoryReq {
                req,
                conn_id,
                res_tx,
            } => {
                self.send_history(req, conn_id).await;
                self.send_response(res_tx, ()).await;
            }
        }
        Ok(())
    }

    #[instrument]
    async fn send_response<T: Debug>(&self, res_tx: oneshot::Sender<T>, result: T) {
        let r = res_tx
            .send(result)
            .map_err(|val| anyhow!("failed to send value in response: {val:?}"));
        log_err_as_error!(r);
    }
}

impl ChatServerHandle {
    /// Register client message sender and obtain connection ID.
    #[instrument(skip())]
    pub async fn register(
        &self,
        conn_tx: mpsc::Sender<Arc<ChatMsg>>,
        user_info: Arc<UserSessionInfo>,
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

    pub fn ws_id(&self) -> WsId {
        self.ws_id
    }

    /// Broadcast message to other users
    #[instrument]
    pub async fn send_msg_to_clients(&self, skip: Option<WsConnId>, msg: ChatMsg) {
        let (res_tx, res_rx) = oneshot::channel();

        self.send_cmd_to_server(Command::ForClients { msg, skip, res_tx }, res_rx)
            .await
            .expect("failed to send command");
    }

    #[instrument]
    pub async fn process_history_request(&self, conn_id: WsConnId, req: ReqHistoryBody) {
        let (res_tx, res_rx) = oneshot::channel();

        self.send_cmd_to_server(
            Command::HistoryReq {
                req,
                conn_id,
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

impl ServerTask for ChatServer {
    fn name(&self) -> &'static str {
        "Chat Server"
    }

    #[instrument(err(Debug))]
    async fn run(mut self, cancellation_token: TrackedCancellationToken) -> anyhow::Result<()> {
        // Ensure that exiting causes the rest of the app to shut down
        let _drop_guard = cancellation_token.clone().drop_guard();
        loop {
            select! {
                _ = cancellation_token.cancelled() => {
                    info!("shutting down ChatServer because of cancellation request");
                    return Ok(())
                }
                cmd = self.cmd_rx.recv() => self.process_cmd(cmd, cancellation_token.clone()).await.context("fatal error in ChatServer, shutting down")?,
            }
        }
    }

    async fn run_without_cancellation(self) -> anyhow::Result<()> {
        unreachable!("run should be used instead as we must check for cancelled to exit")
    }
}
