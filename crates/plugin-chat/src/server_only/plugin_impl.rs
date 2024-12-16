use std::sync::Arc;

use super::server::{ChatServer, ChatServerHandle};
use tracked_cancellations::TrackedCancellationToken;
use ws_auth::WsId;
use wykies_server::plugin::{ServerPlugin, ServerPluginArtifacts};
use wykies_shared::db_types::DbPool;

#[derive(serde::Deserialize, Clone)]
pub struct ChatSettings {
    pub heartbeat_interval_secs: u8,
}

pub struct ChatPluginConfig {
    pub ws_id: WsId,
    pub settings: ChatSettings,
}

pub struct ChatPlugin;

impl ServerPlugin for ChatPlugin {
    type Config = ChatPluginConfig;

    type Task = ChatServer;

    type Handle = ChatServerHandle;

    fn setup(
        config: &Self::Config,
        db_pool: DbPool,
        cancellation_token: TrackedCancellationToken,
        ws_config: &wykies_server::WebSocketSettings,
    ) -> anyhow::Result<wykies_server::plugin::ServerPluginArtifacts<Self::Task, Self::Handle>>
    {
        let (chat_server, chat_server_handle) =
            ChatServer::new(&config.settings, ws_config, db_pool, cancellation_token);
        Ok(ServerPluginArtifacts {
            task: chat_server,
            handle: Arc::new(chat_server_handle),
        })
    }
}
