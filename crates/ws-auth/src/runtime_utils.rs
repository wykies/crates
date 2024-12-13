//! Houses code to make using WebSockets easier and extracts out the boilerplate

use crate::{AuthTokenManager, WebSocketAuthError, WsId};
use actix_web::{dev::ConnectionInfo, web, HttpRequest, HttpResponse};
use anyhow::Context as _;
use wykies_shared::host_branch::HostId;

/// Does a prescreening to see if the request is expected and then starts a WS session to be able to check the token
pub fn pre_screen_incoming_ws_req(
    req: HttpRequest,
    stream: web::Payload,
    conn: ConnectionInfo,
    auth_manager: &AuthTokenManager,
    ws_id: WsId,
) -> Result<
    (
        actix_ws::Session,
        actix_ws::MessageStream,
        HostId,
        HttpResponse,
    ),
    WebSocketAuthError,
> {
    // Validate HostID before attempting to create session
    let client_identifier: HostId = conn.try_into().context("failed to get host_id")?;
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
    Ok((session, msg_stream, client_identifier, res))
}
