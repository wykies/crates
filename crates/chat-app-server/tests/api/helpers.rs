use anyhow::{bail, Context};
use argon2::password_hash::SaltString;
use argon2::PasswordHasher;
use chat_app_server::startup::{start_servers, CustomConfiguration};
use sqlx::{Connection, Executor};
use std::fmt::Debug;
use std::mem::forget;
use std::ops::Deref;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tracked_cancellations::TrackedCancellationToken;
use uuid::Uuid;
use wykies_client_core::LoginOutcome;
use wykies_server::ApiServerBuilder;
use wykies_server::{
    db_types::{DbConnection, DbPool},
    db_utils::validate_one_row_affected,
    get_configuration, get_db_connection_pool, DatabaseSettings,
};
use wykies_shared::const_config::path::PATH_WS_TOKEN_CHAT;
use wykies_shared::{
    host_branch::HostBranchPair,
    id::DbId,
    req_args::LoginReqArgs,
    telemetry::{self, get_subscriber, init_subscriber},
    uac::Username,
};
use wykies_time::Seconds;

const MSG_WAIT_TIMEOUT: Seconds = Seconds::new(2);

// Ensure that the `tracing` stack is only initialised once
pub static TRACING: LazyLock<String> = LazyLock::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let log_file_name = format!("server_tests{}", Uuid::new_v4());
        let (file, path) = telemetry::create_trace_file(&log_file_name).unwrap();
        let subscriber = get_subscriber(subscriber_name, default_filter_level, file);
        init_subscriber(subscriber).unwrap();
        format!("Traces for tests being written to: {path:?}")
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber).unwrap();
        "Traces set to std::io::sink".to_string()
    }
});

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: DbPool,
    pub test_user: TestUser,
    pub core_client: wykies_client_core::Client,
    pub login_attempt_limit: u8,
    pub host_branch_pair: HostBranchPair,
}

impl Debug for TestApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestApp")
            .field("address", &self.address)
            .field("port", &self.port)
            .field("test_user", &self.test_user)
            .finish()
    }
}

impl TestApp {
    /// Creates a clone of [`Self`] with an admin user and separate api_client
    #[tracing::instrument]
    pub async fn create_admin_user(&self) -> Self {
        let admin_user = TestUser::generate("admin");
        admin_user.store(&self.db_pool, true).await;
        Self {
            address: self.address.clone(),
            port: self.port,
            db_pool: self.db_pool.clone(),
            test_user: admin_user,
            core_client: build_core_client(self.address.clone()),
            login_attempt_limit: self.login_attempt_limit,
            host_branch_pair: self.host_branch_pair.clone(),
        }
    }

    #[tracing::instrument]
    pub async fn is_logged_in(&self) -> bool {
        // Also tests if able to establish a websocket connection but this was the simplest alternative that didn't need any permissions
        self.core_client
            .ws_connect(PATH_WS_TOKEN_CHAT, no_cb)
            .await
            .expect("failed to receive on rx")
            .is_ok()
    }

    async fn store_host_branch(&self) {
        let sql_result = sqlx::query!(
            "INSERT INTO `hostbranch` 
            (`hostname`, `AssignedBranch`)
            VALUES (?, ?);",
            self.host_branch_pair.host_id,
            self.host_branch_pair.branch_id
        )
        .execute(&self.db_pool)
        .await
        .unwrap();
        validate_one_row_affected(&sql_result).unwrap();
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

/// Empty function for use when a call back isn't needed
pub fn no_cb() {}

fn build_core_client(server_address: String) -> wykies_client_core::Client {
    wykies_client_core::Client::new(server_address)
}

pub async fn spawn_app() -> TestApp {
    let result = spawn_app_without_host_branch_stored().await;
    result.store_host_branch().await;
    result
}

pub async fn spawn_app_without_host_branch_stored() -> TestApp {
    let (configuration, connection_pool) =
        spawn_app_without_host_branch_stored_before_migration().await;
    do_migrations(&connection_pool).await;
    spawn_app_without_host_branch_stored_after_migration(configuration).await
}

async fn spawn_app_without_host_branch_stored_after_migration(
    configuration: wykies_server::Configuration<CustomConfiguration>,
) -> TestApp {
    let application_port = start_server_in_background(&configuration).await;
    build_test_app(configuration, application_port).await
}

async fn spawn_app_without_host_branch_stored_before_migration(
) -> (wykies_server::Configuration<CustomConfiguration>, DbPool) {
    start_tracing();
    let configuration = get_randomized_configuration();
    let connection_pool = create_database(&configuration.database).await;
    (configuration, connection_pool)
}

async fn build_test_app(
    configuration: wykies_server::Configuration<CustomConfiguration>,
    application_port: u16,
) -> TestApp {
    let login_attempt_limit = configuration.user_auth.login_attempt_limit;
    let db_pool = get_db_connection_pool(&configuration.database);
    let host_branch_pair = HostBranchPair {
        host_id: "127.0.0.1".to_string().try_into().unwrap(),
        branch_id: get_seed_branch_from_db(&db_pool).await,
    };
    let address = format!("http://localhost:{}", application_port);
    let core_client = build_core_client(address.clone());

    let test_app = TestApp {
        address,
        port: application_port,
        db_pool,
        test_user: TestUser::generate("normal"),
        core_client,
        login_attempt_limit,
        host_branch_pair,
    };

    test_app.test_user.store(&test_app.db_pool, false).await;
    test_app
}

async fn start_server_in_background(
    configuration: &wykies_server::Configuration<CustomConfiguration>,
) -> u16 {
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

/// Randomise configuration to ensure test isolation
fn get_randomized_configuration() -> wykies_server::Configuration<CustomConfiguration> {
    let mut c = get_configuration::<CustomConfiguration>().expect("failed to read configuration");
    // Use a different database for each test case
    c.database.database_name = Uuid::new_v4().to_string();
    // Use a random OS port
    c.application.port = 0;
    // Use root user to be able to create a new database
    c.database.username = "root".to_string();
    c
}

fn start_tracing() {
    // Accessing TRACING also forces the LazyLock to initialize
    let logging_msg = TRACING.deref();
    println!("{logging_msg}");
}

#[allow(non_snake_case)]
async fn get_seed_branch_from_db(pool: &DbPool) -> DbId {
    let branch_id = sqlx::query!("SELECT `BranchID` FROM branch LIMIT 1;")
        .fetch_one(pool)
        .await
        .context("failed to get seed branch id")
        .unwrap()
        .BranchID;
    branch_id.try_into().unwrap()
}

async fn create_database(config: &DatabaseSettings) -> DbPool {
    let mut connection = DbConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Database");
    connection
        .execute(&*format!(r#"CREATE DATABASE `{}`;"#, config.database_name))
        .await
        .expect("Failed to create database");

    DbPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Database.")
}

async fn do_migrations(connection_pool: &DbPool) {
    sqlx::migrate!("./migrations")
        .run(connection_pool)
        .await
        .expect("Failed to migrate the database");
}

#[derive(Debug)]
pub struct TestUser {
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate(username_prefix: &str) -> Self {
        let remaining_length = Username::MAX_LENGTH - username_prefix.len() - 1;
        let username = format!(
            "{username_prefix}-{}",
            &Uuid::new_v4().to_string()[..remaining_length]
        );
        Self {
            username,
            password: Uuid::new_v4().to_string(),
        }
    }

    pub fn login_args(&self) -> LoginReqArgs {
        LoginReqArgs::new(self.username.clone(), self.password.clone().into())
    }

    pub async fn disable_in_db(&self, app: &TestApp) {
        let sql_result = sqlx::query!(
            "UPDATE `user` SET `Enabled` = '0' WHERE `user`.`UserName` = ?;",
            self.username,
        )
        .execute(&app.db_pool)
        .await
        .expect("failed to set user to disabled");
        validate_one_row_affected(&sql_result).expect("failed to set user to disabled");
    }

    pub async fn set_locked_out_in_db(&self, app: &TestApp, value: bool) {
        let value = if value { 1 } else { 0 };
        let sql_result = sqlx::query!(
            "UPDATE `user` SET `LockedOut` = ? WHERE `user`.`UserName` = ?;",
            value,
            self.username,
        )
        .execute(&app.db_pool)
        .await
        .expect("failed to set user to disabled");
        validate_one_row_affected(&sql_result).expect("failed to set user to disabled");
    }

    async fn store(&self, pool: &DbPool, is_admin: bool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        // Match production parameters
        let password_hash = wykies_server::authentication::argon2_settings()
            .hash_password(self.password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        if is_admin {
            let sql_result = sqlx::query!(
                "INSERT INTO `roles` 
                (`RoleID`, `Name`, `Description`, `Permissions`, `LockedEditing`) 
                VALUES (NULL, 'Admin', 'Full Permissions', '11111111111111111111111111111111111', '0'); ",
            )
            .execute(pool)
            .await
            .expect("failed to store test user");
            validate_one_row_affected(&sql_result).expect("failed to store admin role");
            let role_id = sql_result.last_insert_id();

            let sql_result = sqlx::query!(
                "INSERT INTO `user`
                (`UserName`, `Password`, `password_hash`, `salt`, `DisplayName`, `AssignedRole`, `PassChangeDate`, `Enabled`) 
                VALUES (?, '', ?, '', 'Admin User', ?, CURRENT_DATE(), 1);",
                self.username,
                password_hash,
                role_id
            )
            .execute(pool)
            .await
            .expect("failed to store test user");
            validate_one_row_affected(&sql_result).expect("failed to store admin user");
        } else {
            let sql_result = sqlx::query!(
                "INSERT INTO `user`
                (`UserName`, `Password`, `password_hash`, `salt`, `DisplayName`, `PassChangeDate`, `Enabled`) 
                VALUES (?, '', ?, '', 'Test User', CURRENT_DATE(), 1);",
                self.username,
                password_hash,
            )
            .execute(pool)
            .await
            .expect("failed to store test user");
            validate_one_row_affected(&sql_result).expect("failed to store test user");
        }
    }
}

pub async fn wait_for_message(
    rx: &ewebsock::WsReceiver,
    should_ignore_ping: bool,
) -> anyhow::Result<ewebsock::WsEvent> {
    let start = Instant::now();
    let timeout: Duration = MSG_WAIT_TIMEOUT.into();
    while start.elapsed() < timeout {
        if let Some(msg) = rx.try_recv() {
            let _empty_vec = Vec::<u8>::new();
            if should_ignore_ping
                && matches!(
                    &msg,
                    ewebsock::WsEvent::Message(ewebsock::WsMessage::Ping(_empty_vec))
                )
            {
                continue; // Skip ping messages
            }
            return Ok(msg);
        } else {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }
    bail!("Timed out after {MSG_WAIT_TIMEOUT:?}")
}
