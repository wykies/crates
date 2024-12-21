use actix_web::web::ServiceConfig;
use anyhow::Context;
use chat_app_server::startup::{start_servers, CustomConfiguration};
use tokio::task::JoinError;
use tracing::{error, info};
use wykies_server::initialize_tracing;
use wykies_server::{cancel_remaining_tasks, ApiServerBuilder, ApiServerInitBundle};
use wykies_shared::telemetry;

#[cfg(feature = "shuttle")]
#[shuttle_runtime::main]
async fn actix_web(
    #[shuttle_shared_db::Postgres] db_pool: sqlx::PgPool,
    #[shuttle_runtime::Secrets] secrets: shuttle_runtime::SecretStore,
) -> shuttle_actix_web::ShuttleActixWeb<impl FnOnce(&mut ServiceConfig) + Send + Clone + 'static> {
    initialize_tracing("chat_app_server", "info", std::io::stdout);

    sqlx::migrate!("./migrations_pg")
        .run(&db_pool)
        .await
        .expect("Migrations failed");

    let ApiServerInitBundle::<CustomConfiguration> {
        cancellation_token,
        cancellation_tracker,
        configuration,
    } = ApiServerInitBundle::new();

    // let api_server_builder = ApiServerBuilder::new(&configuration, db_pool)
    //     .await
    //     .expect("failed to initialize API Server");

    let config = move |cfg: &mut ServiceConfig| {
        // cfg.service(hello_world);
    };

    Ok(config.into())
}

#[cfg(not(feature = "shuttle"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (file, path) = telemetry::create_trace_file("chat-app-server")
        .context("failed to create file for traces")?;
    initialize_tracing("chat_app_server", "info", file);
    println!("Traces being written to: {path:?}");
    let api_server_init_bundle = ApiServerInitBundle::<CustomConfiguration>::new();
    let db_pool =
        wykies_server::get_db_connection_pool(&api_server_init_bundle.configuration.database);
    let api_server_builder = ApiServerBuilder::new(api_server_init_bundle, db_pool)
        .await
        .expect("failed to initialize API Server");

    let addr = wykies_server::get_socket_address(
        &api_server_builder
            .api_server_init_bundle
            .configuration
            .application,
    )
    .context("failed to get socket address")?;
    let (mut join_set, cancellation_tracker, _) = start_servers(api_server_builder, addr).await;
    let join_outcome = join_set.join_next().await.context("no tasks in join set")?;
    report_exit(join_outcome);

    // Cancel any remaining tasks
    cancel_remaining_tasks(cancellation_tracker).await;

    Ok(())
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
