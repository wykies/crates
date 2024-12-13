use std::{future::Future, sync::Arc};

use wykies_shared::session::UserSessionInfo;

pub trait ClientLoopController<WsServerHandle, Output>:
    FnOnce(
    WsServerHandle,
    actix_ws::Session,
    actix_ws::AggregatedMessageStream,
    Arc<UserSessionInfo>,
) -> Output
where
    Output: Future<Output = ()>,
{
}
impl<T, WsServerHandle, Output> ClientLoopController<WsServerHandle, Output> for T
where
    T: FnOnce(
        WsServerHandle,
        actix_ws::Session,
        actix_ws::AggregatedMessageStream,
        Arc<UserSessionInfo>,
    ) -> Output,
    Output: Future<Output = ()>,
{
}
