//! Shared server functionality
//!
//! Selecting which DB will be used is required via Feature flags on this crate

#![warn(unused_crate_dependencies)]

mod configuration;
mod init;
mod macros;
pub mod plugin;
pub mod ws;

#[cfg_attr(feature = "mysql", path = "db_types_mysql.rs")]
pub mod db_types;

pub use configuration::{get_configuration, Configuration, DatabaseSettings, WebSocketSettings};
pub use init::{server_init, ServerInit, ServerTask};
