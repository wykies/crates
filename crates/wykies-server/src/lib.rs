//! Shared server functionality
//!
//! Selecting which DB will be used is required via Feature flags on this crate

#![warn(unused_crate_dependencies)]

#[cfg(all(feature = "disable-cors", not(debug_assertions)))]
mod warning_suppress_release {
    // Needed to prevent warning on release build testing in CI as we force CORS not to be disabled for release builds
    use actix_cors as _;
}

pub mod authentication;
mod configuration;
pub mod db_utils;
mod error_wrappers;
mod macros;
pub mod plugin;
pub mod routes;
mod session_state;
mod startup;
pub mod ws;

#[cfg_attr(feature = "mysql", path = "db_types_mysql.rs")]
pub mod db_types;

pub use configuration::{get_configuration, Configuration, DatabaseSettings, WebSocketSettings};
pub use error_wrappers::{e400, e500};
pub use startup::{get_db_connection_pool, ApiServerBuilder, ApiServerInit, ServerTask};
