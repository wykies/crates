use std::{future::Future, sync::Arc};

use wykies_shared::{host_branch::HostId, session::UserSessionInfo};

pub trait ClientLoopController<WsServerHandle, Output>:
    FnOnce(
    Arc<WsServerHandle>,
    actix_ws::Session,
    actix_ws::AggregatedMessageStream,
    UserSessionInfo,
    HostId,
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
    ) -> Output,
    Output: Future<Output = ()>,
{
}
