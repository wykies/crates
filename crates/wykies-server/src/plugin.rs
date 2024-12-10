use serde::de::DeserializeOwned;
use std::sync::Arc;
use tracked_cancellations::TrackedCancellationToken;

use crate::{db_types::DbPool, ServerTask};

pub struct PluginArtifacts<T, H>
where
    T: ServerTask,
{
    pub task: T,
    pub handle: Arc<H>,
}

pub trait Plugin {
    type ConfigType: DeserializeOwned + Clone;
    type Task: ServerTask;
    type Handle: Send;
    fn name() -> &'static str;
    fn setup(
        config: &Self::ConfigType,
        db_pool: DbPool,
        cancellation_token: TrackedCancellationToken,
    ) -> anyhow::Result<PluginArtifacts<Self::Task, Self::Handle>>;
}
