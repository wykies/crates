use crate::websocket::WsIds;
use actix_web::web::{self, ServiceConfig};
use anyhow::Context;
use plugin_chat::server_only::{
    chat_ws_start_client_handler_loop, ChatPlugin, ChatPluginConfig, ChatSettings,
};
use shuttle_runtime::async_trait;
use tokio::task::{JoinError, JoinSet};
use tracing::{error, info};
use tracked_cancellations::CancellationTracker;
use ws_auth::ws_get_route_add_closures;
use wykies_server::{
    cancel_remaining_tasks,
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
    addr: std::net::SocketAddr,
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
        .build_runnable_api_server(addr, open_resources, protected_resources)
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

pub struct ShuttleService(pub ApiServerBuilder<CustomConfiguration>);

#[async_trait]
impl shuttle_runtime::Service for ShuttleService {
    async fn bind(self, addr: std::net::SocketAddr) -> Result<(), shuttle_runtime::Error> {
        let (mut join_set, cancellation_tracker, _) = start_servers(self.0, addr).await;
        let join_outcome = join_set.join_next().await.context("no tasks in join set")?;
        report_exit(join_outcome);

        // Cancel any remaining tasks
        cancel_remaining_tasks(cancellation_tracker).await;

        Ok(())
    }
}

fn report_exit(join_set_outcome: Result<(&str, Result<anyhow::Result<()>, JoinError>), JoinError>) {
    match join_set_outcome {
        Ok((task_name, spawn_join_outcome)) => match spawn_join_outcome {
            Ok(Ok(())) => info!("{task_name} has exited from the join set with Ok(())"),
            Ok(Err(e)) => {
                error!(
                    error.cause_chain = ?e,
                    error.message = %e,
                    "{task_name} resulted in an error: {e}"
                );
            }
            Err(e) => {
                error!(
                    error.cause_chain = ?e,
                    error.message = %e,
                    "{task_name} resulted in a join error so it must have panicked"
                );
            }
        },
        Err(e) => {
            error!( // Not expected to happen as we have a very small anonymous async function that should not panic
                error.cause_chain = ?e,
                error.message = %e,
                "anonymous async function panicked instead of returning the task name. NO TASK name available"
            );
        }
    }
}
