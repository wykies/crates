use serde::de::DeserializeOwned;
use std::{future::Future, sync::Arc};
use tracked_cancellations::TrackedCancellationToken;

use crate::{db_types::DbPool, ServerRunBundle};

pub struct PluginArtifacts<T, H>
where
    T: Future<Output = ()> + Send + 'static,
{
    pub task: ServerRunBundle<T>,
    pub handle: Arc<H>,
}

pub trait Plugin {
    type ConfigType: DeserializeOwned + Clone;
    type Task: Future<Output = ()> + Send + 'static;
    type Handle: Send;
    fn name() -> &'static str;
    fn setup(
        config: &Self::ConfigType,
        db_pool: DbPool,
        cancellation_token: TrackedCancellationToken,
    ) -> anyhow::Result<PluginArtifacts<Self::Task, Self::Handle>>;
}
