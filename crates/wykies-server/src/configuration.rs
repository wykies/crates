use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::ConnectOptions;
use std::convert::{TryFrom, TryInto};
use wykies_time::Seconds;

use wykies_shared::db_types::{DbConnectOptions, DbSslMode};

// TODO 5: Add comments to any settings that are no longer obvious
#[derive(serde::Deserialize, Clone)]
pub struct Configuration<T: Clone> {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    pub redis_uri: SecretString,
    pub user_auth: UserAuthSettings,
    pub websockets: WebSocketSettings,
    pub custom: T,
}

#[derive(serde::Deserialize, Clone)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    // TODO 2: Look into if this field is used for removal
    pub base_url: String,
    pub hmac_secret: SecretString,
}

#[derive(serde::Deserialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: SecretString,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

#[derive(serde::Deserialize, Clone)]
pub struct UserAuthSettings {
    pub login_attempt_limit: u8,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct WebSocketSettings {
    pub token_lifetime_secs: Seconds,
    pub heartbeat_times_missed_allowance: u8,
    pub heartbeat_additional_buffer_time_secs: Seconds,
}

impl DatabaseSettings {
    pub fn without_db(&self) -> DbConnectOptions {
        #[cfg(feature = "mysql")]
        let ssl_mode = if self.require_ssl {
            DbSslMode::Required
        } else {
            DbSslMode::Preferred
        };
        #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
        let ssl_mode = if self.require_ssl {
            DbSslMode::Require
        } else {
            DbSslMode::Prefer
        };
        DbConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
    }

    pub fn with_db(&self) -> DbConnectOptions {
        let options = self.without_db().database(&self.database_name);
        options.log_statements(tracing::log::LevelFilter::Trace)
    }
}

pub fn get_configuration<T: Clone + DeserializeOwned>(
) -> Result<Configuration<T>, config::ConfigError> {
    let base_path = std::env::current_dir().expect("failed to determine the current directory");

    // Note do not try to move configuration folder to root because it will make
    // it tricky for tests as they start at the crate root not the workspace root
    let configuration_directory = base_path.join("configuration");

    // Detect the running environment.
    // Default to `local` if unspecified.
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("failed to parse APP_ENVIRONMENT.");
    let environment_filename = format!("{}.toml", environment.as_str());
    let settings = config::Config::builder()
        .add_source(config::File::from(
            configuration_directory.join("base.toml"),
        ))
        .add_source(config::File::from(
            configuration_directory.join(environment_filename),
        ))
        // Add in settings from environment variables (with a prefix of APP and '__' as separator)
        // E.g. `APP_APPLICATION__PORT=5001 would set `Settings.application.port`
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    settings.try_deserialize::<Configuration<T>>()
}

/// The possible runtime environment for our application.
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either `local` or `production`.",
                other
            )),
        }
    }
}
