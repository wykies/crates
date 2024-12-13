use actix_web::{dev::ConnectionInfo, web, HttpRequest, HttpResponse};
use tokio::task::spawn_local;
use ws_auth::{create_ws_session, AuthTokenManager, WebSocketAuthError, WsId};

use super::{client_control_loop::chat_ws_start_client_handler_loop, server::ChatServerHandle};

/// Handshake and start WebSocket handler with heartbeats.
#[tracing::instrument(skip(stream))]
pub async fn chat_ws_start_session(
    req: HttpRequest,
    stream: web::Payload,
    chat_server_handle: web::Data<ChatServerHandle>,
    auth_manager: web::Data<AuthTokenManager>,
    conn: ConnectionInfo,
    ws_id: WsId,
) -> Result<HttpResponse, WebSocketAuthError> {
    let (session, msg_stream, client_identifier, res) =
        create_ws_session(req, stream, conn, &auth_manager, ws_id)?;

    // spawn websocket handler (don't await) so response is sent immediately
    spawn_local(chat_ws_start_client_handler_loop(
        (**chat_server_handle).clone(),
        session,
        msg_stream,
        auth_manager,
        client_identifier,
        ws_id,
    ));

    Ok(res)
}
