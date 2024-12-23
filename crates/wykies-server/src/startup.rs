use crate::{
    authentication::{validate_user_access, LoginAttemptLimit},
    configuration::ApplicationSettings,
    get_configuration,
    routes::{
        branch_create, branch_list, change_password, health_check, host_branch_pair_lookup,
        list_host_branch_pairs, list_users_and_roles, log_out, login, not_found, password_reset,
        role, role_assign, role_create, set_host_branch_pair, status, user, user_new, user_update,
    },
    Configuration, DatabaseSettings,
};
#[cfg(all(not(feature = "redis-session-rustls"), feature = "cookie-session"))]
use actix_session::storage::CookieSessionStore;
#[cfg(feature = "redis-session-rustls")]
use actix_session::storage::RedisSessionStore;
use actix_session::SessionMiddleware;
use actix_web::{
    middleware::from_fn,
    web::{self, ServiceConfig},
    App, HttpResponse, HttpServer,
};
use anyhow::Context as _;
use secrecy::ExposeSecret as _;
use serde::de::DeserializeOwned;
use std::{
    future::Future,
    net::{SocketAddr, TcpListener},
    str::FromStr,
};
use tracing::{info, instrument};
use tracing_actix_web::TracingLogger;
use tracked_cancellations::{CancellationTracker, TrackedCancellationToken};
use ws_auth::AuthTokenManager;
use wykies_shared::{
    const_config,
    db_types::{DbPool, DbPoolOptions},
    telemetry,
};

pub trait ServerTask {
    fn name(&self) -> &'static str;

    fn run(
        self,
        cancellation_token: TrackedCancellationToken,
    ) -> impl Future<Output = anyhow::Result<()>> + Send
    where
        Self: Sized + Send;
}

/// Bundles the information used to start a server
pub struct ApiServerInitBundle<T: Clone> {
    pub cancellation_token: TrackedCancellationToken,
    pub cancellation_tracker: CancellationTracker,
    pub configuration: Configuration<T>,
}

pub struct ApiServerBuilder<T>
where
    T: Clone,
{
    pub db_pool: DbPool,
    pub api_server_init_bundle: ApiServerInitBundle<T>,
    pkg_version: &'static str,
}

/// Initializes Tracing
pub fn initialize_tracing<Sink, D, N>(
    subscriber_name: N,
    default_env_filter_directive: D,
    sink: Sink,
) where
    Sink: for<'b> tracing_subscriber::fmt::MakeWriter<'b> + Send + Sync + 'static,
    D: AsRef<str>,
    N: Into<String>,
{
    let subscriber =
        telemetry::get_subscriber(subscriber_name.into(), default_env_filter_directive, sink);
    telemetry::init_subscriber(subscriber).expect("failed to initialize the subscriber");
}

pub struct RunnableApiServer(actix_web::dev::Server);

impl<T> ApiServerInitBundle<T>
where
    T: Clone + DeserializeOwned,
{
    /// Does the initial prep before starting to build the server
    pub fn new() -> ApiServerInitBundle<T> {
        let (cancellation_token, cancellation_tracker) = TrackedCancellationToken::new();
        let configuration = get_configuration::<T>().expect("failed to read configuration.");

        ApiServerInitBundle {
            cancellation_token,
            cancellation_tracker,
            configuration,
        }
    }
}

impl<T> Default for ApiServerInitBundle<T>
where
    T: Clone + DeserializeOwned,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + DeserializeOwned> ApiServerBuilder<T> {
    pub async fn new(
        api_server_init_bundle: ApiServerInitBundle<T>,
        db_pool: DbPool,
        pkg_version: &'static str,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            db_pool,
            api_server_init_bundle,
            pkg_version,
        })
    }

    #[instrument(err(Debug), skip_all)]
    pub async fn build_runnable_api_server<FOpen, FProtected>(
        self,
        addr: std::net::SocketAddr,
        open_resource: FOpen,
        protected_resource: FProtected,
    ) -> anyhow::Result<(RunnableApiServer, CancellationTracker, u16)>
    where
        FOpen: Fn(&mut ServiceConfig) + Send + Clone + 'static,
        FProtected: Fn(&mut ServiceConfig) + Send + Clone + 'static,
    {
        let db_pool = web::Data::new(self.db_pool);
        let ApiServerInitBundle {
            cancellation_tracker,
            configuration,
            ..
        } = self.api_server_init_bundle;

        let login_attempt_limit = web::Data::new(LoginAttemptLimit(
            configuration.user_auth.login_attempt_limit,
        ));

        let websocket_auth_manager = web::Data::new(AuthTokenManager::new(
            configuration.websockets.token_lifetime_secs,
        ));

        let secret_key = actix_web::cookie::Key::from(
            configuration
                .application
                .hmac_secret
                .expose_secret()
                .as_bytes(),
        );
        let pkg_version = self.pkg_version;

        #[cfg(feature = "redis-session-rustls")]
        let session_store = {
            let redis_store = RedisSessionStore::new(configuration.redis_uri.expose_secret())
                .await
                .expect("failed to connect to Redis");
            info!(
                session_store = "RedisSessionStore",
                "Successfully connected to Redis"
            );
            redis_store
        };

        let listener = TcpListener::bind(addr)
            .with_context(|| format!("failed to bind to address: {}", addr))?;
        let port = listener
            .local_addr()
            .context("failed to get local address of listener")?
            .port();
        info!(?port, "Port assigned to the server is {port}");

        let server = HttpServer::new(move || {
            let app = App::new();

            // If both a debug build and disable-cors flag is set then set CORS to
            // permissive. This code runs once per thread so several of this will be
            // printed if cors is disabled
            #[cfg(all(debug_assertions, feature = "disable-cors"))]
            let cors = actix_cors::Cors::permissive();
            #[cfg(all(debug_assertions, feature = "disable-cors"))]
            let app = app.wrap(cors);
            #[cfg(all(debug_assertions, feature = "disable-cors"))]
            {
                tracing::warn!("CORS set to permissive");
                eprintln!("CORS set to permissive");
            }

            #[cfg(all(not(feature = "redis-session-rustls"), feature = "cookie-session"))]
            let session_store = {
                info!(
                    // This info is repeated for each thread but less bad than duplicating the cfg
                    session_store = "CookieSessionStore",
                    "Using Cookie Only Session Storage"
                );
                CookieSessionStore::default()
            };

            #[cfg(feature = "redis-session-rustls")]
            let session_store = session_store.clone(); // When using redis we need to clone

            // TODO 4: Look into session expiration https://docs.rs/actix-session/latest/actix_session/config/struct.SessionMiddlewareBuilder.html

            let front_end_folder = if cfg!(feature = "running-from-workspace-root") {
                "./crates/chat-app-server/app/"
            } else {
                "./app/"
            };

            app.wrap(SessionMiddleware::new(session_store, secret_key.clone()))
                .wrap(TracingLogger::default())
                .service(
                    web::scope("/api")
                        .wrap(from_fn(validate_user_access))
                        .configure(protected_resource.clone())
                        .route("/change_password", web::post().to(change_password))
                        .route("/logout", web::post().to(log_out))
                        .service(
                            web::scope("/admin")
                                .service(
                                    web::scope("/branch")
                                        .route("/create", web::post().to(branch_create)),
                                )
                                .service(
                                    web::scope("/host_branch")
                                        .route("/list", web::get().to(list_host_branch_pairs))
                                        .route("/set", web::post().to(set_host_branch_pair)),
                                )
                                .service(
                                    web::scope("/role")
                                        .route("/", web::get().to(role))
                                        .route("/assign", web::post().to(role_assign))
                                        .route("/create", web::post().to(role_create)),
                                )
                                .service(
                                    web::scope("/user")
                                        .route("/", web::get().to(user))
                                        .route("/list", web::get().to(list_users_and_roles))
                                        .route("/new", web::post().to(user_new))
                                        .route("/password_reset", web::post().to(password_reset))
                                        .route("/update", web::post().to(user_update)),
                                ),
                        )
                        .route(
                            "/host_branch/lookup",
                            web::get().to(host_branch_pair_lookup),
                        ),
                )
                .configure(open_resource.clone())
                .route("/branches", web::get().to(branch_list))
                .route("/health_check", web::get().to(health_check))
                .route("/login", web::post().to(login))
                .route("/status", web::get().to(status))
                .route(
                    "/version",
                    web::get().to(|| async { HttpResponse::Ok().body(pkg_version.to_string()) }),
                )
                .service(actix_files::Files::new("/", front_end_folder).index_file("index.html"))
                .app_data(db_pool.clone())
                .app_data(login_attempt_limit.clone())
                .app_data(websocket_auth_manager.clone())
                .default_service(web::route().to(not_found))
        })
        .listen(listener)
        .context("Failed to bind HTTP Server to listener")?
        .run();
        info!(
            version = env!("CARGO_PKG_VERSION"),
            "API Server prepared to be run at version {}", pkg_version
        );
        Ok((RunnableApiServer(server), cancellation_tracker, port))
    }
}

impl ServerTask for RunnableApiServer {
    fn name(&self) -> &'static str {
        "API Server"
    }

    async fn run(self, cancellation_token: TrackedCancellationToken) -> anyhow::Result<()> {
        let _guard = cancellation_token.drop_guard();
        self.0.await.context("api server crashed")
    }
}
#[instrument(skip_all)]
pub fn get_db_connection_pool(database_config: &DatabaseSettings) -> DbPool {
    #[cfg(feature = "mysql")]
    info!("DB in set is MySql on port: {}", database_config.port);
    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    info!("DB in set is Postgres on port: {}", database_config.port);

    DbPoolOptions::new()
        .acquire_timeout(const_config::server::DB_ACQUIRE_TIMEOUT.into())
        .connect_lazy_with(database_config.with_db())
}

pub fn get_socket_address(app_config: &ApplicationSettings) -> anyhow::Result<SocketAddr> {
    let address = format!("{}:{}", app_config.host, app_config.port);
    info!("Socket address to parse is: {address}");
    let result = SocketAddr::from_str(&address)
        .with_context(|| format!("failed to parse address: {address}"))?;
    Ok(result)
}
