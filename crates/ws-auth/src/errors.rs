use crate::WsId;
use wykies_shared::host_branch::HostId;

#[derive(thiserror::Error, Debug)]
pub enum WebSocketError {
    /// Client was not expected to be trying to connect
    #[error("Unexpected Client")]
    UnexpectedClient {
        client_identifier: HostId,
        ws_id: WsId,
    },
    #[error("Invalid Token")]
    InvalidToken {
        client_identifier: HostId,
        ws_id: WsId,
    },
    #[error("Unable to start session")]
    FailedToStartSession(anyhow::Error),
    #[error("Unexpected Error")]
    UnexpectedError(#[from] anyhow::Error),
}

#[cfg(not(target_arch = "wasm32"))]
pub mod conversions {
    use super::*;
    use actix_web::http::StatusCode;

    impl actix_web::error::ResponseError for WebSocketError {
        fn status_code(&self) -> StatusCode {
            match self {
                WebSocketError::UnexpectedClient { .. } => StatusCode::BAD_REQUEST,
                WebSocketError::InvalidToken { .. } => StatusCode::BAD_REQUEST,
                WebSocketError::FailedToStartSession(_) => StatusCode::INTERNAL_SERVER_ERROR,
                WebSocketError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            }
        }
    }
}
