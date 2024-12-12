use std::sync::Arc;

use actix_web::{dev::ConnectionInfo, web, HttpRequest, HttpResponse};
use anyhow::Context as _;
use tokio::task::spawn_local;
use ws_auth::{AuthTokenManager, WebSocketAuthError, WsId};
use wykies_server::e500;
use wykies_shared::{host_branch::HostId, session::UserSessionInfo, token::AuthToken};

use super::{client_control_loop::chat_ws_start_client_handler_loop, server::ChatServerHandle};

/// Handshake and start WebSocket handler with heartbeats.
#[tracing::instrument(skip(stream))]
pub async fn chat_ws_start_session(
    req: HttpRequest,
    stream: web::Payload,
    chat_server_handle: web::Data<ChatServerHandle>,
    auth_manager: web::Data<AuthTokenManager>,
    conn: ConnectionInfo,
) -> Result<HttpResponse, WebSocketAuthError> {
    // Validate HostID before attempting to create session
    let client_identifier: HostId = conn.try_into().context("failed to get host_id")?;
    let ws_id = chat_server_handle.ws_id();
    if !auth_manager.is_expected_host(&client_identifier, ws_id) {
        return Err(WebSocketAuthError::UnexpectedClient {
            client_identifier,
            ws_id,
        });
    }

    // Create a new websocket session
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)
        .map_err(|e| anyhow::anyhow!("{e:?}"))
        .map_err(WebSocketAuthError::FailedToStartSession)?;

    // spawn websocket handler (don't await) so response is sent immediately
    spawn_local(chat_ws_start_client_handler_loop(
        (**chat_server_handle).clone(),
        session,
        msg_stream,
        auth_manager,
        client_identifier,
    ));

    Ok(res)
}

#[tracing::instrument(ret, err(Debug))]
pub async fn chat_get_token(
    auth_manager: web::Data<AuthTokenManager>,
    conn: ConnectionInfo,
    user_info: web::ReqData<UserSessionInfo>,
) -> actix_web::Result<web::Json<AuthToken>> {
    let result = AuthToken::new_rand();
    let host_id: HostId = conn
        .try_into()
        .context("failed to get host_id")
        .map_err(e500)?;
    auth_manager.record_token(
        host_id,
        WsId::new(1), // TODO 1: This needs to be provided by ws-auth
        Arc::new(user_info.into_inner()),
        result.clone(),
    );
    Ok(web::Json(result))
}
