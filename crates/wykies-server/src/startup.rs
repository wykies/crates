use crate::{
    authentication::{validate_user_access, LoginAttemptLimit},
    db_types::{DbPool, DbPoolOptions},
    get_configuration,
    routes::{
        branch_create, branch_list, change_password, health_check, host_branch_pair_lookup,
        list_host_branch_pairs, list_users_and_roles, log_out, login, not_found, password_reset,
        role, role_assign, role_create, set_host_branch_pair, status, user, user_new, user_update,
    },
    Configuration, DatabaseSettings,
};
use actix_session::{storage::RedisSessionStore, SessionMiddleware};
use actix_web::{
    middleware::from_fn,
    web::{self, ServiceConfig},
    App, HttpServer,
};
use anyhow::Context as _;
use secrecy::ExposeSecret as _;
use serde::de::DeserializeOwned;
use std::{future::Future, net::TcpListener};
use tracing::info;
use tracing_actix_web::TracingLogger;
use tracked_cancellations::{CancellationTracker, TrackedCancellationToken};
use ws_auth::AuthTokenManager;
use wykies_shared::{const_config, telemetry};

pub trait ServerTask {
    fn name(&self) -> &'static str;

    fn run(
        self,
        cancellation_token: TrackedCancellationToken,
    ) -> impl Future<Output = anyhow::Result<()>> + Send
    where
        Self: Sized + Send,
    {
        async move {
            // Ensure that exiting causes the rest of the app to shut down
            let _drop_guard = cancellation_token.clone().drop_guard();
            self.run_without_cancellation().await
        }
    }

    #[doc(hidden)]
    /// Meant to be called from `run` or if you really don't want automatic cancellation support
    /// When Implementing use `async fn run_without_cancellation(self) -> anyhow::Result<()>` instead.
    /// Didn't use async syntax below to be able to add bounds
    fn run_without_cancellation(self) -> impl Future<Output = anyhow::Result<()>> + Send;
}

/// Bundles the information used to start a server
pub struct ServerInit<T: Clone> {
    pub cancellation_token: TrackedCancellationToken,
    pub cancellation_tracker: CancellationTracker,
    pub configuration: Configuration<T>,
}

pub struct ServerBuilder<'a, T>
where
    T: Clone,
{
    pub db_pool: DbPool,
    configuration: &'a Configuration<T>,
    listener: TcpListener,
}

pub struct RunnableServer(actix_web::dev::Server);

impl<T> ServerInit<T>
where
    T: Clone + DeserializeOwned,
{
    /// Does the initial prep before starting to build the server
    pub fn new_with_tracing_init<Sink, D, N>(
        subscriber_name: N,
        default_env_filter_directive: D,
        sink: Sink,
    ) -> ServerInit<T>
    where
        Sink: for<'b> tracing_subscriber::fmt::MakeWriter<'b> + Send + Sync + 'static,
        D: AsRef<str>,
        N: Into<String>,
    {
        let subscriber =
            telemetry::get_subscriber(subscriber_name.into(), default_env_filter_directive, sink);
        telemetry::init_subscriber(subscriber).expect("failed to initialize the subscriber");
        let (cancellation_token, cancellation_tracker) = TrackedCancellationToken::new();
        let configuration = get_configuration::<T>().expect("failed to read configuration.");

        ServerInit {
            cancellation_token,
            cancellation_tracker,
            configuration,
        }
    }
}

impl<'a, T> ServerBuilder<'a, T>
where
    T: Clone + DeserializeOwned,
{
    pub async fn new(configuration: &'a Configuration<T>) -> anyhow::Result<Self> {
        let db_pool = get_db_connection_pool(&configuration.database);
        let listener = get_listener(configuration).context("failed to register listener")?;
        Ok(Self {
            db_pool,
            configuration,
            listener,
        })
    }

    pub fn port(&self) -> anyhow::Result<u16> {
        Ok(self
            .listener
            .local_addr()
            .context("failed to access local address")?
            .port())
    }

    pub async fn finish<FOpen, FProtected>(
        self,
        open_resource: FOpen,
        protected_resource: FProtected,
    ) -> anyhow::Result<RunnableServer>
    where
        FOpen: Fn(&mut ServiceConfig) + Send + Clone + Copy + 'static,
        FProtected: Fn(&mut ServiceConfig) + Send + Clone + Copy + 'static,
    {
        let db_pool = web::Data::new(self.db_pool);

        let login_attempt_limit = web::Data::new(LoginAttemptLimit(
            self.configuration.user_auth.login_attempt_limit,
        ));

        let websocket_auth_manager = web::Data::new(AuthTokenManager::new(
            self.configuration.websockets.token_lifetime_secs,
        ));

        let secret_key = actix_web::cookie::Key::from(
            self.configuration
                .application
                .hmac_secret
                .expose_secret()
                .as_bytes(),
        );

        let redis_store = RedisSessionStore::new(self.configuration.redis_uri.expose_secret())
            .await
            .context("failed to connect to Redis")?;
        info!("Successfully connected to Redis");

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

            app.wrap(SessionMiddleware::new(
                redis_store.clone(),
                secret_key.clone(),
            ))
            .wrap(TracingLogger::default())
            .service(
                web::scope("/api")
                    .wrap(from_fn(validate_user_access))
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
                            )
                            .configure(protected_resource),
                    )
                    .route(
                        "/host_branch/lookup",
                        web::get().to(host_branch_pair_lookup),
                    )
                    .configure(open_resource),
            )
            .route("/login", web::post().to(login))
            .route("/branches", web::get().to(branch_list))
            .route("/health_check", web::get().to(health_check))
            .route("/status", web::get().to(status))
            .service(actix_files::Files::new("/", "./app/").index_file("index.html"))
            .app_data(db_pool.clone())
            .app_data(login_attempt_limit.clone())
            .app_data(websocket_auth_manager.clone())
            .default_service(web::route().to(not_found))
        })
        .listen(self.listener)
        .context("Failed to bind HTTP Server to listener")?
        .run();
        Ok(RunnableServer(server))
    }
}

impl ServerTask for RunnableServer {
    fn name(&self) -> &'static str {
        "API Server"
    }

    async fn run_without_cancellation(self) -> anyhow::Result<()> {
        self.0.await.context("api server crashed")
    }
}

pub fn get_db_connection_pool(database_config: &DatabaseSettings) -> DbPool {
    DbPoolOptions::new()
        .acquire_timeout(const_config::server::DB_ACQUIRE_TIMEOUT.into())
        .connect_lazy_with(database_config.with_db())
}

fn get_listener<T: Clone>(configuration: &Configuration<T>) -> anyhow::Result<TcpListener> {
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    info!("Server address is: {address}");
    println!("Server status page address is: http://{address}/status"); // Provides feedback on stdout that server is starting
    println!("Client app being served at http://{address}/",);
    #[cfg(debug_assertions)]
    println!(
        "To avoid CORS issues during testing use this link http://localhost:{}/",
        configuration.application.port
    );
    let listener = TcpListener::bind(&address)
        .with_context(|| format!("failed to bind to address: {address}"))?;
    let port = listener
        .local_addr()
        .context("failed to access local address")?
        .port();
    info!(?port, "Port assigned to the server is {port}");
    Ok(listener)
}
