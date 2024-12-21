use crate::websocket::WsIds;
use actix_web::web::{self, ServiceConfig};
use plugin_chat::server_only::{
    chat_ws_start_client_handler_loop, ChatPlugin, ChatPluginConfig, ChatSettings,
};
use tokio::task::{JoinError, JoinSet};
use tracing::info;
use tracked_cancellations::CancellationTracker;
use ws_auth::ws_get_route_add_closures;
use wykies_server::{
    plugin::{ServerPlugin, ServerPluginArtifacts},
    ApiServerBuilder, ServerTask as _,
};
use wykies_shared::uac::init_permissions_to_defaults;

#[derive(Clone, serde::Deserialize)]
pub struct CustomConfiguration {
    pub chat: ChatSettings,
}

pub async fn start_servers(
    api_server_builder: ApiServerBuilder<CustomConfiguration>,
) -> (
    JoinSet<(&'static str, Result<anyhow::Result<()>, JoinError>)>,
    CancellationTracker,
    u16,
) {
    init_permissions_to_defaults();

    let configuration = &api_server_builder.api_server_init_bundle.configuration;
    let cancellation_token = api_server_builder
        .api_server_init_bundle
        .cancellation_token
        .clone();

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
    let (api_server, cancellation_tacker, port) = api_server_builder
        .build_runnable_api_server(open_resources, protected_resources)
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

    (result, cancellation_tacker, port)
}
