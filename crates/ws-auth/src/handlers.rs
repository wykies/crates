use crate::{
    AuthTokenManager, ClientLoopController, WebSocketAuthError, WsServiceId,
    runtime_utils::{
        pre_screen_incoming_ws_req, validate_connection_then_start_client_handler_loop,
    },
};
use actix_web::{
    HttpRequest, HttpResponse,
    dev::ConnectionInfo,
    web::{self, ServiceConfig},
};
use anyhow::Context as _;
use std::{future::Future, sync::Arc};
use tokio::task::spawn_local;
use wykies_shared::{e500, host_branch::HostId, token::AuthToken, uac::UserInfo};
use wykies_time::Seconds;

#[tracing::instrument(ret, err(Debug))]
pub async fn get_ws_token(
    auth_manager: web::Data<AuthTokenManager>,
    conn: ConnectionInfo,
    user_info: web::ReqData<UserInfo>,
    ws_id: WsServiceId,
) -> actix_web::Result<web::Json<AuthToken>> {
    let result = AuthToken::new_rand();
    let host_id: HostId = conn
        .try_into()
        .context("failed to get host_id")
        .map_err(e500)?;
    auth_manager.record_token(host_id, ws_id, user_info.into_inner(), result.clone());
    Ok(web::Json(result))
}

/// Handshake and start WebSocket handler
#[expect(clippy::too_many_arguments)] // All arguments are well typed, no material benefit from creating a type
#[tracing::instrument(skip(stream, ws_start_client_handler_loop, ws_server_handle))]
pub async fn ws_start_session<WsServerHandle, Output>(
    req: HttpRequest,
    stream: web::Payload,
    ws_server_handle: web::Data<WsServerHandle>,
    auth_manager: web::Data<AuthTokenManager>,
    conn: ConnectionInfo,
    ws_id: WsServiceId,
    initial_msg_timeout: Seconds,
    ws_start_client_handler_loop: impl ClientLoopController<WsServerHandle, Output> + 'static,
) -> Result<HttpResponse, WebSocketAuthError>
where
    Output: Future<Output = ()> + 'static,
    WsServerHandle: 'static,
{
    let (session, msg_stream, client_identifier, res) =
        pre_screen_incoming_ws_req(req, stream, conn, &auth_manager, ws_id)?;

    // spawn websocket handler (don't await) so response is sent immediately
    spawn_local(validate_connection_then_start_client_handler_loop(
        Arc::clone(&ws_server_handle.into_inner()),
        session,
        msg_stream,
        auth_manager,
        client_identifier,
        ws_id,
        initial_msg_timeout,
        ws_start_client_handler_loop,
    ));

    Ok(res)
}

pub fn ws_get_route_add_closures<WsServerHandle, Output>(
    name: &'static str,
    ws_id: WsServiceId,
    initial_msg_timeout: Seconds,
    ws_start_client_handler_loop: impl ClientLoopController<WsServerHandle, Output> + 'static + Clone,
) -> (
    impl Fn(&mut ServiceConfig) + 'static + Clone,
    impl Fn(&mut ServiceConfig) + 'static + Clone,
)
where
    Output: Future<Output = ()> + 'static,
    WsServerHandle: Clone + 'static,
{
    let open_handler = move |req: HttpRequest,
                             stream: web::Payload,
                             ws_server_handle: web::Data<WsServerHandle>,
                             auth_manager: web::Data<AuthTokenManager>,
                             conn: ConnectionInfo| {
        ws_start_session(
            req,
            stream,
            ws_server_handle,
            auth_manager,
            conn,
            ws_id,
            initial_msg_timeout,
            ws_start_client_handler_loop.clone(),
        )
    };
    let ws_open_add = move |cfg: &mut ServiceConfig| {
        cfg.route(name, web::get().to(open_handler.clone()));
    };
    let protected_handler = move |auth_manager: web::Data<AuthTokenManager>,
                                  conn: ConnectionInfo,
                                  user_info: web::ReqData<UserInfo>| {
        get_ws_token(auth_manager, conn, user_info, ws_id)
    };
    let ws_protected_add = move |cfg: &mut ServiceConfig| {
        cfg.route(name, web::post().to(protected_handler));
    };
    (ws_open_add, ws_protected_add)
}
