use chat_app_server::startup::CustomConfiguration;
use chat_app_server::startup::ShuttleService;

use wykies_server::initialize_tracing;
use wykies_server::{ApiServerBuilder, ApiServerInitBundle};

// Includes the not standalone for when all features are enabled by CI
#[cfg(all(feature = "shuttle", not(feature = "standalone")))]
#[shuttle_runtime::main]
async fn main(
    #[shuttle_shared_db::Postgres] db_pool: sqlx::PgPool,
    #[shuttle_runtime::Secrets] secrets: shuttle_runtime::SecretStore,
) -> Result<ShuttleService, shuttle_runtime::Error> {
    initialize_tracing("chat_app_server", "info", std::io::stdout);

    sqlx::migrate!("./migrations_pg")
        .run(&db_pool)
        .await
        .expect("Migrations failed");

    let api_server_init_bundle = ApiServerInitBundle::<CustomConfiguration>::new();
    let api_server_builder = ApiServerBuilder::new(api_server_init_bundle, db_pool)
        .await
        .expect("failed to initialize API Server");

    Ok(ShuttleService(api_server_builder))
}

#[cfg(feature = "standalone")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use anyhow::Context as _;
    use shuttle_runtime::Service as _;

    let (file, path) = wykies_shared::telemetry::create_trace_file("chat-app-server")
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

    ShuttleService(api_server_builder)
        .bind(addr)
        .await
        .context("service runtime error")
}
