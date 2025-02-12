use anyhow::Context as _;
use chat_app_server::startup::AppService;
use chat_app_server::startup::CustomConfiguration;
use wykies_server::initialize_tracing;
use wykies_server::{ApiServerBuilder, ApiServerInitBundle};

// Includes the not standalone for when all features are enabled by CI
#[cfg(all(feature = "shuttle", not(feature = "standalone")))]
#[shuttle_runtime::main]
async fn main(
    #[shuttle_shared_db::Postgres(
        local_uri = "postgres://db_user:{secrets.DB_PASSWORD}@localhost:5432/chat_demo"
    )]
    db_pool: sqlx::PgPool,
    #[shuttle_runtime::Secrets] secrets: shuttle_runtime::SecretStore,
) -> Result<AppService, shuttle_runtime::Error> {
    initialize_tracing("chat_app_server", "info", std::io::stdout);

    sqlx::migrate!("./migrations_pg")
        .run(&db_pool)
        .await
        .expect("Migrations failed");

    let mut api_server_init_bundle = ApiServerInitBundle::<CustomConfiguration>::new();
    api_server_init_bundle.configuration.application.hmac_secret =
        secrecy::SecretBox::from(secrets.get("HMAC").context("hmac secret was not found")?);
    let api_server_builder =
        ApiServerBuilder::new(api_server_init_bundle, db_pool, env!("CARGO_PKG_VERSION"))
            .await
            .expect("failed to initialize API Server");

    tracing::info!("Setup Completed");
    Ok(AppService(api_server_builder))
}

#[cfg(feature = "standalone")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use shuttle_runtime::Service as _;

    let (writer, path, _guard) = wykies_shared::telemetry::setup_tracing_writer("chat-app-server")
        .context("failed to setup traces")?;
    initialize_tracing("chat_app_server", "info", writer);
    println!(
        "Traces being written to: {:?}",
        path.canonicalize()
            .context("trace file canonicalization failed")?
    );
    let api_server_init_bundle = ApiServerInitBundle::<CustomConfiguration>::new();
    let db_pool =
        wykies_server::get_db_connection_pool(&api_server_init_bundle.configuration.database);
    let api_server_builder =
        ApiServerBuilder::new(api_server_init_bundle, db_pool, env!("CARGO_PKG_VERSION"))
            .await
            .expect("failed to initialize API Server");

    let addr = wykies_server::get_socket_address(
        &api_server_builder
            .api_server_init_bundle
            .configuration
            .application,
    )
    .context("failed to get socket address")?;

    AppService(api_server_builder)
        .bind(addr)
        .await
        .context("service runtime error")
}
