//! Shared server functionality
//!
//! Selecting which DB will be used is required via Feature flags on this crate

#![warn(unused_crate_dependencies)]

#[cfg(all(feature = "disable-cors", not(debug_assertions)))]
mod warning_suppress_release {
    // Needed to prevent warning on release build testing in CI as we force CORS not
    // to be disabled for release builds
    use actix_cors as _;
}

#[cfg(all(not(feature = "mysql"), not(feature = "postgres")))]
compile_error!("At least one database must be selected using feature flags");

pub mod authentication;
mod configuration;
pub mod db_utils;
pub mod plugin;
pub mod routes;
mod session_state;
mod startup;
pub mod ws;

pub use configuration::{get_configuration, Configuration, DatabaseSettings, WebSocketSettings};
pub use startup::{
    get_db_connection_pool, get_socket_address, initialize_tracing, ApiServerBuilder,
    ApiServerInitBundle, ServerTask,
};
use tracked_cancellations::CancellationTracker;
use wykies_shared::const_config::server::SERVER_SHUTDOWN_TIMEOUT;

pub async fn cancel_remaining_tasks(mut cancellation_tracker: CancellationTracker) {
    cancellation_tracker.cancel();
    cancellation_tracker
        .await_cancellations(SERVER_SHUTDOWN_TIMEOUT.into())
        .await;
}
