//! Code related to the loop that handles incoming and outgoing messages to the client (Outgoing messages include those from other threads)

use crate::{ChatIM, ChatMsg};

use super::server::ChatServerHandle;
use actix_web::web;
use actix_ws::{AggregatedMessage, CloseCode, CloseReason, ProtocolError, Session};
use anyhow::{bail, Context};
use futures_util::StreamExt as _;
use std::{pin::pin, sync::Arc, time::Instant};
use tokio::{select, sync::mpsc};
use tracing::{debug, error, info, instrument, warn, Span};
use ws_auth::{validate_ws_connection, AuthTokenManager, WsConnId};
use wykies_shared::{
    const_config::CHANNEL_BUFFER_SIZE, debug_panic, host_branch::HostId, log_err_as_error,
};
use wykies_time::Timestamp;

#[instrument(skip(session, msg_stream), fields(ws_conn_id))]
pub async fn chat_ws_start_client_handler_loop(
    chat_server_handle: ChatServerHandle,
    mut session: actix_ws::Session,
    msg_stream: actix_ws::MessageStream,
    auth_manager: web::Data<AuthTokenManager>,
    client_identifier: HostId,
) {
    let (user_info, msg_stream) = match validate_ws_connection(
        msg_stream,
        auth_manager,
        &client_identifier,
        chat_server_handle.ws_id(),
    )
    .await
    {
        Ok(msg_stream) => msg_stream,
        Err(e) => {
            // Connection not validated exit
            error!("Failed to validate web socket connection with error: {e:?}");
            let _ = session.close(Some(CloseCode::Error.into())).await;
            debug_panic!(e);
            return;
        }
    };

    let mut last_heartbeat = Instant::now();
    let mut heartbeat_interval = chat_server_handle.heartbeat_config.interval();

    let (conn_tx, mut conn_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

    let (conn_id, cancellation_token) = chat_server_handle.register(conn_tx, user_info).await;
    Span::current().record("ws_conn_id", format!("{conn_id:?}"));
    info!("Chat connected for {conn_id:?}");

    let mut msg_stream = pin!(msg_stream);

    let close_reason = loop {
        select! {
            // Handle request for cancellation
            _ = cancellation_token.cancelled() => {
                info!("Received cancellation request. Closing Connection");
                break CloseCode::Away.into();
            }

            // Handle heartbeat ticks
            _ = heartbeat_interval.tick() => {
                if let Some(reason) = process_heartbeat_tick(last_heartbeat, &chat_server_handle, &mut session).await{
                    break reason;
                };
            }

            chat_msg = conn_rx.recv() => {
                if let Some(reason) = process_message_from_server(chat_msg, &mut session).await{
                    break reason;
                }
            }

            stream_msg = msg_stream.next() => {
                if let Some(reason) = process_stream_msg(
                    stream_msg,
                    &mut last_heartbeat,
                    &mut session,
                    &chat_server_handle,
                    &conn_id
                ).await {
                    break reason;
                }
            }
        }
    };

    info!(
        ?close_reason,
        "Connection to client closed because of close_reason: {close_reason:?}"
    );

    if !matches!(
        close_reason,
        CloseReason {
            code: CloseCode::Away,
            ..
        }
    ) {
        // Only try to unregister if the server is still around
        chat_server_handle.unregister(conn_id).await;
    }

    // attempt to close connection gracefully
    let _ = session.close(Some(close_reason)).await;
}

#[instrument(skip(session))]
/// Handle chat messages received - From Server like chat from other users
async fn process_message_from_server(
    msg: Option<Arc<ChatMsg>>,
    session: &mut Session,
) -> Option<CloseReason> {
    match msg {
        Some(chat_msg) => {
            let r = session
                .text(serde_json::to_string(&chat_msg).expect("failed to serialize msg"))
                .await
                .context("failed to send text msg");
            log_err_as_error!(r);
            None
        }

        // Server dropped the sender, it has been shutdown
        None => Some(CloseCode::Away.into()),
    }
}

#[instrument(skip(session))]
/// Handle stream messages received - commands & messages received from client
async fn process_stream_msg(
    msg: Option<Result<AggregatedMessage, ProtocolError>>,
    last_heartbeat: &mut Instant,
    session: &mut Session,
    chat_server: &ChatServerHandle,
    conn_id: &WsConnId,
) -> Option<CloseReason> {
    match msg {
        // Message from remote client
        Some(Ok(msg)) => {
            debug!("msg: {msg:?}");

            match msg {
                AggregatedMessage::Ping(bytes) => {
                    *last_heartbeat = Instant::now();
                    let r = session.pong(&bytes).await.context("failed to send pong");
                    log_err_as_error!(r);
                    None
                }

                AggregatedMessage::Pong(_) => {
                    *last_heartbeat = Instant::now();
                    None
                }

                AggregatedMessage::Text(text) => {
                    let r = process_msg_from_client(chat_server, &text, conn_id.clone()).await;
                    log_err_as_error!(r);
                    None
                }

                AggregatedMessage::Binary(_bin) => {
                    warn!("unexpected binary message being ignored");
                    Some(CloseCode::Unsupported.into())
                }

                AggregatedMessage::Close(reason) => {
                    info!("Received close message from client with reason: {reason:?}");
                    Some(CloseCode::Normal.into())
                }
            }
        }

        // client WebSocket stream error
        Some(Err(err)) => {
            error!("{:?}", err);
            debug_panic!(err);
            Some(CloseReason {
                code: CloseCode::Error,
                description: Some(err.to_string()),
            })
        }

        // client WebSocket stream ended
        None => Some(CloseCode::Normal.into()),
    }
}

#[instrument(level = "debug", skip(session))]
/// if no heartbeat ping/pong received recently, close the connection
async fn process_heartbeat_tick(
    last_heartbeat: Instant,
    chat_server: &ChatServerHandle,
    session: &mut Session,
) -> Option<CloseReason> {
    if Instant::now().duration_since(last_heartbeat) > chat_server.heartbeat_config.client_timeout()
    {
        info!(
            "client has not sent heartbeat in over {}; disconnecting",
            chat_server.heartbeat_config.client_timeout_display()
        );
        return Some(CloseReason {
            code: CloseCode::Policy,
            description: Some("Failed to respond to ping".into()),
        });
    }

    // send heartbeat ping
    let r = session.ping(b"").await.context("failed to send ping");
    log_err_as_error!(r);
    None
}

#[instrument]
async fn process_msg_from_client(
    chat_server: &ChatServerHandle,
    text: &str,
    conn_id: WsConnId,
) -> anyhow::Result<()> {
    let chat_msg: ChatMsg =
        serde_json::from_str(text).context("failed to deserialize chat msg received")?;

    match chat_msg {
        ChatMsg::UserJoined(_)
        | ChatMsg::UserLeft(_)
        | ChatMsg::InitialState(_)
        | ChatMsg::RespHistory(_) => {
            bail!("unexpected message type received from the client of: {chat_msg:?}")
        }
        ChatMsg::IM(mut chat_im) => {
            validate_im_from_client(&mut chat_im, &conn_id).context("IM validation failed")?;

            // Provide no skip so that author also receives the message to get the correct
            // timestamp
            chat_server
                .send_msg_to_clients(None, ChatMsg::IM(chat_im))
                .await;
        }
        ChatMsg::ReqHistory(req) => chat_server.process_history_request(conn_id, req).await,
    }
    Ok(())
}

fn validate_im_from_client(im: &mut ChatIM, conn_id: &WsConnId) -> anyhow::Result<()> {
    im.timestamp = Timestamp::now(); // Replace timestamp with server time to ensure monotonicity

    if im.author != conn_id.user_info.username {
        error!(
            "unexpected message author (Author reset to expected). Expected '{}' Found: '{}'",
            conn_id.user_info.username, im.author,
        );
        debug_panic!("user name doesn't match");
        im.author = conn_id.user_info.username.clone();
    }

    Ok(())
}
