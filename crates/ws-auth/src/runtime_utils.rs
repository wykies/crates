//! Houses code to make using WebSockets easier and extracts out the boilerplate

use std::{future::Future, sync::Arc};

use crate::{
    validate_ws_connection, AuthTokenManager, ClientLoopController, WebSocketAuthError, WsId,
};
use actix_web::{dev::ConnectionInfo, web, HttpRequest, HttpResponse};
use actix_ws::CloseCode;
use anyhow::Context as _;
use tracing::{error, instrument};
use wykies_shared::{debug_panic, host_branch::HostId};

/// Does a prescreening to see if the request is expected and then starts a WS session to be able to check the token
#[instrument(err, skip(stream))]
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

#[instrument(skip(session, msg_stream, ws_server_handle, ws_start_client_handler_loop))]
pub async fn validate_connection_then_start_client_handler_loop<WsServerHandle, Output>(
    ws_server_handle: Arc<WsServerHandle>,
    session: actix_ws::Session,
    msg_stream: actix_ws::MessageStream,
    auth_manager: web::Data<AuthTokenManager>,
    client_identifier: HostId,
    ws_id: WsId,
    ws_start_client_handler_loop: impl ClientLoopController<WsServerHandle, Output>,
) where
    Output: Future<Output = ()>,
{
    let (user_info, msg_stream) =
        match validate_ws_connection(msg_stream, auth_manager, &client_identifier, ws_id).await {
            Ok(value) => value,
            Err(e) => {
                // Connection not validated exit
                error!("Failed to validate web socket connection with error: {e:?}");
                let _ = session.close(Some(CloseCode::Error.into())).await;
                debug_panic!(e);
                return;
            }
        };

    ws_start_client_handler_loop(ws_server_handle, session, msg_stream, user_info).await;
}
