use crate::{permissions::init_permissions, websocket::WsIds};
use actix_web::web::{self, ServiceConfig};
use plugin_chat::server_only::{
    chat_ws_start_client_handler_loop, ChatPlugin, ChatPluginConfig, ChatSettings,
};
use serde::de::DeserializeOwned;
use tokio::task::{JoinError, JoinSet};
use tracing::info;
use tracked_cancellations::TrackedCancellationToken;
use ws_auth::ws_get_route_add_closures;
use wykies_server::{
    plugin::{ServerPlugin, ServerPluginArtifacts},
    ApiServerBuilder, Configuration, ServerTask as _,
};

#[derive(Clone, serde::Deserialize)]
pub struct CustomConfiguration {
    pub chat: ChatSettings,
}

pub async fn start_servers<T>(
    api_server_builder: ApiServerBuilder<'_, T>,
    configuration: &Configuration<CustomConfiguration>,
    cancellation_token: TrackedCancellationToken,
) -> JoinSet<(&'static str, Result<anyhow::Result<()>, JoinError>)>
where
    T: Clone + DeserializeOwned,
{
    init_permissions();

    // Chat Server
    let ServerPluginArtifacts {
        task: chat_server,
        handle: chat_server_handle,
    } = ChatPlugin::setup(
        &ChatPluginConfig {
            ws_id: WsIds::CHAT,
            settings: configuration.custom.chat.clone(),
        },
        api_server_builder.db_pool.clone(),
        cancellation_token.clone(),
        &configuration.websockets,
    )
    .expect("failed to start Chat Server");

    // Setup Routes / Server Resources
    let (chat_open_add, chat_protected_add) =
        ws_get_route_add_closures("chat", WsIds::CHAT, chat_ws_start_client_handler_loop);
    let open_resources = move |cfg: &mut ServiceConfig| {
        cfg.service(web::scope("/ws").configure(chat_open_add.clone()))
            .app_data(web::Data::from(chat_server_handle.clone()));
    };
    let protected_resources = move |cfg: &mut ServiceConfig| {
        cfg.service(web::scope("/ws_token").configure(chat_protected_add.clone()));
    };

    // Finalize Server
    let api_server = api_server_builder
        .finish(open_resources, protected_resources)
        .await
        .expect("failed to finalize API Server");

    // Start up the tasks
    let mut result = JoinSet::new();
    let cancellation_token1 = cancellation_token.clone();
    result.spawn(async move {
        let name = api_server.name();
        (
            name,
            tokio::spawn(api_server.run(cancellation_token1)).await,
        )
    });
    result.spawn(async move {
        let name = chat_server.name();
        (
            name,
            tokio::spawn(chat_server.run(cancellation_token)).await,
        )
    });

    // Print a message to stdout that server is started
    println!("-- Server Started --");
    info!("-- Server Started --");
    println!("{}", "-".repeat(80)); // Add separator

    result
}
