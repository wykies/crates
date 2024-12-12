use crate::{db_types::DbPool, ServerTask, WebSocketSettings};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tracked_cancellations::TrackedCancellationToken;

pub struct ServerPluginArtifacts<T, H>
where
    T: ServerTask,
{
    pub task: T,
    pub handle: Arc<H>,
}

pub trait ServerPlugin {
    type Config: DeserializeOwned + Clone;
    type Task: ServerTask;
    type Handle: Send;

    /// The `cancellation_token` is to be used for any other tasks that they spin up
    /// The token for the plugin itself will be passed when the ServerTask is run
    fn setup(
        config: &Self::Config,
        db_pool: DbPool,
        cancellation_token: TrackedCancellationToken,
        ws_config: &WebSocketSettings,
    ) -> anyhow::Result<ServerPluginArtifacts<Self::Task, Self::Handle>>;
}
