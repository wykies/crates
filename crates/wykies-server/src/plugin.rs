use serde::de::DeserializeOwned;
use std::{future::Future, sync::Arc};
use tracked_cancellations::TrackedCancellationToken;

use crate::db_types::DbPool;

pub struct PluginRunBundle<T>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    pub name: &'static str,
    pub task: T,
}

pub struct PluginArtifacts<T, H>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    pub task: PluginRunBundle<T>,
    pub handle: Arc<H>,
}

pub trait Plugin {
    type ConfigType: DeserializeOwned + Clone;
    type Task: Future + Send + 'static;
    type Handle: Send;
    fn name() -> &'static str;
    fn setup(
        config: &Self::ConfigType,
        db_pool: DbPool,
        cancellation_token: TrackedCancellationToken,
    ) -> anyhow::Result<PluginArtifacts<Self::Task, Self::Handle>>
    where
        <Self::Task as Future>::Output: Send + 'static;
}
