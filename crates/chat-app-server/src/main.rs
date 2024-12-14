use anyhow::Context;
use chat_app_server::startup::{start_servers, CustomConfiguration};
use tokio::task::JoinError;
use tracing::{error, info};
use wykies_server::{cancel_remaining_tasks, ApiServerBuilder, ApiServerInit};
use wykies_shared::telemetry;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Prep to start building server
    let (file, path) = telemetry::create_trace_file("chat-app-server")
        .context("failed to create file for traces")?;
    println!("Traces being written to: {path:?}");
    let ApiServerInit::<CustomConfiguration> {
        cancellation_token,
        cancellation_tracker,
        configuration,
    } = ApiServerInit::new_with_tracing_init("chat_app_server", "info", file);

    let api_server_builder = ApiServerBuilder::new(&configuration)
        .await
        .expect("failed to initialize API Server");

    let mut join_set = start_servers(api_server_builder, &configuration, cancellation_token).await;
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
