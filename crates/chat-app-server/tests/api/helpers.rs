use chat_app_server::startup::{start_servers, CustomConfiguration};
use std::{
    mem::forget,
    ops::{Deref, DerefMut},
};
use tracked_cancellations::TrackedCancellationToken;
use wykies_client_core::LoginOutcome;
use wykies_server::{ApiServerBuilder, ApiServerInitBundle, Configuration};
use wykies_server_test_helper::{
    build_test_app, convert_port_to_test_address,
    spawn_app_without_host_branch_stored_before_migration, store_host_branch, TestUser,
};
use wykies_shared::{const_config::path::PATH_WS_TOKEN_CHAT, db_types::DbPool};

pub use wykies_server_test_helper::{no_cb, wait_for_message};

#[derive(Debug)]
pub struct TestApp(wykies_server_test_helper::TestApp<wykies_client_core::Client>);

impl Deref for TestApp {
    type Target = wykies_server_test_helper::TestApp<wykies_client_core::Client>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TestApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub async fn spawn_app() -> TestApp {
    let result = spawn_app_without_host_branch_stored().await;
    store_host_branch(&result).await;
    result
}

pub async fn spawn_app_without_host_branch_stored() -> TestApp {
    let (configuration, db_pool) =
        spawn_app_without_host_branch_stored_before_migration::<CustomConfiguration>().await;
    do_migrations(&db_pool).await;
    let application_port = start_server_in_background(configuration.clone(), db_pool).await;
    TestApp(
        build_test_app(
            configuration,
            convert_port_to_test_address(application_port),
            wykies_client_core::Client::new,
        )
        .await,
    )
}

async fn do_migrations(connection_pool: &DbPool) {
    #[cfg(feature = "mysql")]
    let migrator = sqlx::migrate!("./migrations_mysql");

    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    let migrator = sqlx::migrate!("./migrations_pg");

    migrator
        .run(connection_pool)
        .await
        .expect("Failed to migrate the database");
}

async fn start_server_in_background(
    configuration: Configuration<CustomConfiguration>,
    db_pool: DbPool,
) -> u16 {
    // Prepare to start server
    let (cancellation_token, cancellation_tracker) = TrackedCancellationToken::new();

    let api_server_init_bundle = ApiServerInitBundle {
        cancellation_token,
        cancellation_tracker,
        configuration,
    };

    let api_server_builder = ApiServerBuilder::new(api_server_init_bundle, db_pool)
        .await
        .expect("Failed to build application.");
    let addr = wykies_server::get_socket_address(
        &api_server_builder
            .api_server_init_bundle
            .configuration
            .application,
    )
    .expect("failed to get socket address");
    let (join_set, _cancellation_tracker, port) = start_servers(api_server_builder, addr).await;
    // Leak the JoinSet so the server doesn't get shutdown
    forget(join_set);
    port
}

impl TestApp {
    /// Creates a clone of [`Self`] with an admin user and separate api_client
    #[tracing::instrument]
    pub async fn create_admin_user(&self) -> Self {
        let admin_user = TestUser::generate("admin");
        admin_user.store(&self.db_pool, true).await;
        Self(
            wykies_server_test_helper::TestApp::<wykies_client_core::Client> {
                address: self.address.clone(),
                db_pool: self.db_pool.clone(),
                test_user: admin_user,
                core_client: wykies_client_core::Client::new(self.address.clone()),
                login_attempt_limit: self.login_attempt_limit,
                host_branch_pair: self.host_branch_pair.clone(),
            },
        )
    }

    #[tracing::instrument]
    pub async fn is_logged_in(&self) -> bool {
        // Also tests if able to establish a websocket connection but this was the
        // simplest alternative that didn't need any permissions
        self.core_client
            .ws_connect(PATH_WS_TOKEN_CHAT, no_cb)
            .await
            .expect("failed to receive on rx")
            .is_ok()
    }

    pub async fn login(&self) -> anyhow::Result<LoginOutcome> {
        self.core_client
            .login(self.test_user.login_args(), no_cb)
            .await
            .unwrap()
    }

    /// Logs in the user and panics if the login is not successful
    pub async fn login_assert(&self) {
        assert!(self
            .core_client
            .login(self.test_user.login_args(), no_cb)
            .await
            .expect("failed to receive on rx")
            .expect("failed to extract login outcome")
            .is_any_success());
    }

    /// Logs out the user and panics on errors
    pub async fn logout_assert(&self) {
        self.core_client
            .logout(no_cb)
            .await
            .expect("failed to receive on rx")
            .expect("login result was not ok");
    }
}
