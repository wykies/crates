use crate::{get_configuration, Configuration};
use serde::de::DeserializeOwned;
use std::future::Future;
use tracked_cancellations::{CancellationTracker, TrackedCancellationToken};
use wykies_shared::telemetry;

pub struct ServerInit<T: Clone> {
    pub cancellation_token: TrackedCancellationToken,
    pub cancellation_tracker: CancellationTracker,
    pub configuration: Configuration<T>,
}

// TODO 5: Add example to docstring of calling with stdout
/// Does the initial prep before starting to build the server
pub fn server_init<Sink, S, T>(default_env_filter_directive: S, sink: Sink) -> ServerInit<T>
where
    Sink: for<'a> tracing_subscriber::fmt::MakeWriter<'a> + Send + Sync + 'static,
    S: AsRef<str>,
    T: Clone + DeserializeOwned,
{
    let (cancellation_token, cancellation_tracker) = TrackedCancellationToken::new();
    let subscriber =
        telemetry::get_subscriber("wic_server".into(), default_env_filter_directive, sink);
    telemetry::init_subscriber(subscriber).expect("failed to initialize the subscriber");
    let configuration = get_configuration::<T>().expect("failed to read configuration.");
    ServerInit {
        cancellation_token,
        cancellation_tracker,
        configuration,
    }
}

pub trait ServerTask {
    fn name() -> &'static str;

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

    /// Meant to be called from `run` or if you really don't want automatic cancellation support
    fn run_without_cancellation(self) -> impl Future<Output = anyhow::Result<()>> + Send;
}
