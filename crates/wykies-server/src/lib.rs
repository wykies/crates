//! Shared server functionality
//!
//! Selecting which DB will be used is required via Feature flags on this crate

#![warn(unused_crate_dependencies)]

use tracked_cancellations::{CancellationTracker, TrackedCancellationToken};
use wykies_shared::telemetry;

mod configuration;
mod macros;

#[cfg_attr(feature = "mysql", path = "db_types_mysql.rs")]
pub mod db_types;

pub use configuration::{get_configuration, Configuration, DatabaseSettings, WebSocketSettings};

pub struct ServerInit {
    pub cancellation_token: TrackedCancellationToken,
    pub cancellation_tracker: CancellationTracker,
    pub configuration: Configuration,
}

// TODO 5: Add example to docstring of calling with stdout
/// Does the initial prep before starting to build the server
pub fn server_init<Sink, S>(default_env_filter_directive: S, sink: Sink) -> ServerInit
where
    Sink: for<'a> tracing_subscriber::fmt::MakeWriter<'a> + Send + Sync + 'static,
    S: AsRef<str>,
{
    let (cancellation_token, cancellation_tracker) = TrackedCancellationToken::new();
    let subscriber =
        telemetry::get_subscriber("wic_server".into(), default_env_filter_directive, sink);
    telemetry::init_subscriber(subscriber).expect("failed to initialize the subscriber");
    let configuration = get_configuration().expect("failed to read configuration.");
    ServerInit {
        cancellation_token,
        cancellation_tracker,
        configuration,
    }
}
