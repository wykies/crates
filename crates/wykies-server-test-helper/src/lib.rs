#![warn(unused_crate_dependencies)]

use anyhow::{bail, Context};
use argon2::password_hash::SaltString;
use argon2::PasswordHasher;
use serde::de::DeserializeOwned;
use sqlx::{Connection, Executor};
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use uuid::Uuid;
use wykies_server::Configuration;
use wykies_server::{
    db_utils::validate_one_row_affected, get_configuration, get_db_connection_pool,
    DatabaseSettings,
};
use wykies_shared::{
    db_types::{DbConnection, DbPool},
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

pub struct TestApp<C> {
    pub address: String,
    pub db_pool: DbPool,
    pub test_user: TestUser,
    pub core_client: C,
    pub login_attempt_limit: u8,
    pub host_branch_pair: HostBranchPair,
}

impl<C> Debug for TestApp<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestApp")
            .field("address", &self.address)
            .field("test_user", &self.test_user)
            .finish()
    }
}

/// Empty function for use when a call back isn't needed
pub fn no_cb() {}

pub async fn spawn_app_without_host_branch_stored_before_migration<T>() -> (Configuration<T>, DbPool)
where
    T: Clone + DeserializeOwned,
{
    start_tracing();
    let configuration = get_randomized_configuration();
    let connection_pool = create_database(&configuration.database).await;
    (configuration, connection_pool)
}

pub fn port_to_test_address(application_port: u16) -> String {
    format!("http://localhost:{application_port}")
}

pub async fn build_test_app<T, C, F>(
    configuration: Configuration<T>,
    address: String,
    build_client: F,
) -> TestApp<C>
where
    T: Clone + DeserializeOwned,
    F: FnOnce(String) -> C,
{
    let login_attempt_limit = configuration.user_auth.login_attempt_limit;
    let db_pool = get_db_connection_pool(&configuration.database);
    let host_branch_pair = HostBranchPair {
        host_id: "127.0.0.1".to_string().try_into().unwrap(),
        branch_id: get_seed_branch_from_db(&db_pool).await,
    };
    let core_client = build_client(address.clone());

    let test_app = TestApp {
        address,
        db_pool,
        test_user: TestUser::generate("normal"),
        core_client,
        login_attempt_limit,
        host_branch_pair,
    };

    test_app.test_user.store(&test_app.db_pool, false).await;
    test_app
}

/// Randomise configuration to ensure test isolation
fn get_randomized_configuration<T>() -> Configuration<T>
where
    T: Clone + DeserializeOwned,
{
    let mut c = get_configuration::<T>().expect("failed to read configuration");
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

async fn get_seed_branch_from_db(pool: &DbPool) -> DbId {
    #[cfg(feature = "mysql")]
    let branch_id = sqlx::query!("SELECT `BranchID` FROM branch LIMIT 1;")
        .fetch_one(pool)
        .await
        .context("failed to get seed branch id")
        .unwrap()
        .BranchID;
    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    let branch_id = sqlx::query!("SELECT branch_id FROM branch LIMIT 1;")
        .fetch_one(pool)
        .await
        .context("failed to get seed branch id")
        .unwrap()
        .branch_id;
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

    pub async fn disable_in_db<C>(&self, app: &TestApp<C>) {
        let sql_result = sqlx::query!(
            "UPDATE `user` SET `Enabled` = '0' WHERE `user`.`UserName` = ?;",
            self.username,
        )
        .execute(&app.db_pool)
        .await
        .expect("failed to set user to disabled");
        validate_one_row_affected(&sql_result).expect("failed to set user to disabled");
    }

    pub async fn set_locked_out_in_db<C>(&self, app: &TestApp<C>, value: bool) {
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

    pub async fn store(&self, pool: &DbPool, is_admin: bool) {
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

pub async fn store_host_branch<C>(test_app: &TestApp<C>) {
    let sql_result = sqlx::query!(
        "INSERT INTO `hostbranch` 
        (`hostname`, `AssignedBranch`)
        VALUES (?, ?);",
        test_app.host_branch_pair.host_id,
        test_app.host_branch_pair.branch_id
    )
    .execute(&test_app.db_pool)
    .await
    .unwrap();
    validate_one_row_affected(&sql_result).unwrap();
}
