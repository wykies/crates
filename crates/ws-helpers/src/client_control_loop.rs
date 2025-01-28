use crate::heartbeat::HeartbeatMonitor;
use actix_ws::{AggregatedMessage, CloseCode, CloseReason, ProtocolError, Session};
use anyhow::Context;
use bytestring::ByteString;
use std::fmt::Debug;
use tracing::{debug, error, info, instrument, warn};
use wykies_shared::{debug_panic, log_err_as_error};

pub enum StreamOutcome {
    MsgFromClient(ByteString),
    CloseSession(CloseReason),
    None,
}
#[instrument(skip(ws_session))]
/// Handle stream messages received - commands & messages received from client
pub async fn process_stream_from_client(
    msg: Option<Result<AggregatedMessage, ProtocolError>>,
    heartbeat: &mut HeartbeatMonitor,
    ws_session: &mut Session,
) -> StreamOutcome {
    match msg {
        // Message from remote client
        Some(Ok(msg)) => {
            // TODO 4: See if this just duplicates the msg as it's one of the arguments
            debug!(?msg, "Message received from client");

            match msg {
                AggregatedMessage::Ping(bytes) => {
                    heartbeat.response_received();
                    let r = ws_session.pong(&bytes).await.context("failed to send pong");
                    log_err_as_error!(r);
                    StreamOutcome::None
                }

                AggregatedMessage::Pong(_) => {
                    heartbeat.response_received();
                    StreamOutcome::None
                }

                AggregatedMessage::Text(text) => StreamOutcome::MsgFromClient(text),

                AggregatedMessage::Binary(_bin) => {
                    warn!("unexpected binary message. Closing connection");
                    StreamOutcome::CloseSession(CloseCode::Unsupported.into())
                }

                AggregatedMessage::Close(reason) => {
                    info!("Received close message from client with reason: {reason:?}");
                    StreamOutcome::CloseSession(CloseCode::Normal.into())
                }
            }
        }

        // client WebSocket stream error
        Some(Err(err_msg)) => {
            error!(?err_msg, "Protocol error with websocket connection");
            debug_panic!(err_msg);
            StreamOutcome::CloseSession(CloseReason {
                code: CloseCode::Error,
                description: Some(err_msg.to_string()),
            })
        }

        // client WebSocket stream ended
        None => StreamOutcome::CloseSession(CloseCode::Normal.into()),
    }
}

#[instrument(skip(session))]
/// Forward messages received from the server to client
pub async fn send_message_to_client<T>(
    server_msg: Option<T>,
    session: &mut Session,
) -> Option<CloseReason>
where
    T: serde::Serialize + Debug,
{
    match server_msg {
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
