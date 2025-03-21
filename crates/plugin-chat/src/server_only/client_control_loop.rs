//! Code related to the loop that handles incoming and outgoing messages to the
//! client (Outgoing messages include those from other threads)

use super::ChatServerHandle;
use crate::{ChatIM, ChatMsg};
use actix_ws::{CloseCode, CloseReason};
use anyhow::{bail, Context};
use futures_util::StreamExt as _;
use std::{pin::pin, sync::Arc};
use tokio::{select, sync::mpsc};
use tracing::{info, instrument, Span};
use ws_helpers::client_control_loop::{
    process_stream_from_client, send_message_to_client, StreamOutcome,
};
use wykies_shared::{
    const_config::CHANNEL_BUFFER_SIZE,
    debug_panic,
    host_branch::HostId,
    log_err_as_error,
    uac::{UserInfo, Username},
    websockets::WsConnId,
};
use wykies_time::{Seconds, Timestamp};

#[instrument(skip(ws_session, msg_stream, chat_server_handle), fields(request_id))]
pub async fn chat_ws_start_client_handler_loop(
    chat_server_handle: Arc<ChatServerHandle>,
    mut ws_session: actix_ws::Session,
    msg_stream: actix_ws::AggregatedMessageStream,
    user_info: UserInfo,
    _host_id: HostId,
    initial_msg_timeout: Seconds,
) {
    let mut heartbeat = chat_server_handle.heartbeat_config.start_new_monitor();
    let username = user_info.username.clone();

    let (conn_tx, mut conn_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

    let (conn_id, cancellation_token) = chat_server_handle.register(conn_tx, user_info).await;
    Span::current().record("request_id", conn_id.inner_as_string());
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
            _ = heartbeat.tick() => {
                if let Some(reason) = heartbeat.process_tick(&mut ws_session).await{
                    break reason;
                };
            }

            server_msg = conn_rx.recv() => {
                if let Some(reason) = send_message_to_client(server_msg, &mut ws_session).await{
                    break reason;
                }
            }

            stream_msg = msg_stream.next() => {
                let outcome = process_stream_from_client(stream_msg, &mut heartbeat, &mut ws_session).await;
                match outcome{
                    StreamOutcome::MsgFromClient(msg) => {
                        let r = process_msg_from_client(&chat_server_handle, &msg, &conn_id, &username).await;
                        log_err_as_error!(r);
                    },
                    StreamOutcome::CloseSession(close_reason) => break close_reason,
                    StreamOutcome::None => {},
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
    let _ = ws_session.close(Some(close_reason)).await;
}

#[instrument]
async fn process_msg_from_client(
    chat_server: &ChatServerHandle,
    text: &str,
    conn_id: &WsConnId,
    username: &Username,
) -> anyhow::Result<()> {
    let chat_msg: ChatMsg =
        serde_json::from_str(text).context("failed to deserialize chat msg received")?;

    match chat_msg {
        ChatMsg::UserJoined(_)
        | ChatMsg::UserLeft(_)
        | ChatMsg::InitialState(_)
        | ChatMsg::RespHistory(_) => {
            bail!("unexpected message type received from the client: {chat_msg:?}")
        }
        ChatMsg::IM(mut chat_im) => {
            validate_im_from_client(&mut chat_im, username).context("IM validation failed")?;

            // Also send to original author so they receive the correct timestamp
            chat_server.send_msg_to_clients(ChatMsg::IM(chat_im)).await;
        }
        ChatMsg::ReqHistory(req) => chat_server.process_history_request(conn_id, req).await,
    }
    Ok(())
}

fn validate_im_from_client(im: &mut ChatIM, username: &Username) -> anyhow::Result<()> {
    im.timestamp = Timestamp::now(); // Replace timestamp with server time to ensure monotonicity

    if &im.author != username {
        debug_panic!("unexpected message author found. Author has been reset to expected value. Expected '{}' Found: '{}'",
            username, im.author,);
        im.author = username.clone();
    }

    Ok(())
}
