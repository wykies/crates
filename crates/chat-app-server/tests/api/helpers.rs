use chat_app_server::startup::{start_servers, CustomConfiguration};
use std::mem::forget;
use tracked_cancellations::TrackedCancellationToken;
use wykies_server::{db_types::DbPool, ApiServerBuilder, Configuration};
use wykies_server_test_helper::{
    build_test_app, spawn_app_without_host_branch_stored_before_migration, store_host_branch,
};

pub use wykies_server_test_helper::{no_cb, wait_for_message, TestApp};

pub async fn spawn_app() -> TestApp {
    let result = spawn_app_without_host_branch_stored().await;
    store_host_branch(&result).await;
    result
}

pub async fn spawn_app_without_host_branch_stored() -> TestApp {
    let (configuration, connection_pool) =
        spawn_app_without_host_branch_stored_before_migration::<CustomConfiguration>().await;
    do_migrations(&connection_pool).await;
    let application_port = start_server_in_background(&configuration).await;
    build_test_app(configuration, application_port).await
}

async fn do_migrations(connection_pool: &DbPool) {
    sqlx::migrate!("./migrations")
        .run(connection_pool)
        .await
        .expect("Failed to migrate the database");
}

async fn start_server_in_background(configuration: &Configuration<CustomConfiguration>) -> u16 {
    // Create tokens to be able to start server
    let (cancellation_token, _) = TrackedCancellationToken::new();

    // Launch the application as a background task
    let server_builder = ApiServerBuilder::new(configuration)
        .await
        .expect("Failed to build application.");
    let application_port = server_builder
        .port()
        .expect("failed to get application port");
    let join_set = start_servers(server_builder, configuration, cancellation_token).await;
    forget(join_set);
    // Leak the JoinSet so the server doesn't get shutdown
    application_port
}
