use actix_web::rt::task::JoinHandle;
use anyhow::Context;
use std::{fs::create_dir_all, path::PathBuf};
use tracing::subscriber::set_global_default;
use tracing::Subscriber;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

/// Compose multiple layers into a `tracing`'s subscriber.
///
/// For details acceptable Filter Directives see <https://docs.rs/tracing-subscriber/0.3.19/tracing_subscriber/filter/struct.EnvFilter.html#directives>
///
/// # Implementation Notes
///
/// We are using `impl Subscriber` as return type to avoid having to spell out
/// the actual type of the returned subscriber, which is indeed quite complex.
pub fn get_subscriber<Sink, S>(
    name: String,
    default_env_filter_directive: S,
    sink: Sink,
) -> impl Subscriber + Sync + Send
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
    S: AsRef<str>,
{
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_env_filter_directive));
    let formatting_layer = BunyanFormattingLayer::new(name, sink);
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

/// Register a subscriber as global default to process span data.
///
/// It should only be called once!
pub fn init_subscriber(subscriber: impl Subscriber + Sync + Send) -> anyhow::Result<()> {
    LogTracer::init().context("Failed to set logger")?;
    set_global_default(subscriber).context("Failed to set subscriber")?;
    Ok(())
}

pub fn spawn_blocking_with_tracing<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = tracing::Span::current();
    actix_web::rt::task::spawn_blocking(move || current_span.in_scope(f))
}

/// Returns a handle to the file created and the file path
pub fn setup_tracing_writer(app_name: &str) -> anyhow::Result<(NonBlocking, PathBuf, WorkerGuard)> {
    // Create logging folder
    let mut log_folder = PathBuf::from("traces");
    log_folder.push(app_name);
    create_dir_all(&log_folder).context("Failed to create logging folder")?;

    // Start non blocking logger wrapping a rolling logger
    let file_appender = tracing_appender::rolling::daily(&log_folder, app_name);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    Ok((non_blocking, log_folder, guard))
}
