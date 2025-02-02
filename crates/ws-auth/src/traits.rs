use std::{future::Future, sync::Arc};
use wykies_shared::{host_branch::HostId, session::UserSessionInfo};
use wykies_time::Seconds;

pub trait ClientLoopController<WsServerHandle, Output>:
    FnOnce(
    Arc<WsServerHandle>,
    actix_ws::Session,
    actix_ws::AggregatedMessageStream,
    UserSessionInfo,
    HostId,
    Seconds,
) -> Output
where
    Output: Future<Output = ()>,
{
}
impl<T, WsServerHandle, Output> ClientLoopController<WsServerHandle, Output> for T
where
    T: FnOnce(
        Arc<WsServerHandle>,
        actix_ws::Session,
        actix_ws::AggregatedMessageStream,
        UserSessionInfo,
        HostId,
        Seconds,
    ) -> Output,
    Output: Future<Output = ()>,
{
}
