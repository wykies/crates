use crate::WsId;
use wykies_shared::host_branch::HostId;

#[derive(thiserror::Error, Debug)]
pub enum WebSocketAuthError {
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

    impl actix_web::error::ResponseError for WebSocketAuthError {
        fn status_code(&self) -> StatusCode {
            match self {
                WebSocketAuthError::UnexpectedClient { .. } => StatusCode::BAD_REQUEST,
                WebSocketAuthError::InvalidToken { .. } => StatusCode::BAD_REQUEST,
                WebSocketAuthError::FailedToStartSession(_) => StatusCode::INTERNAL_SERVER_ERROR,
                WebSocketAuthError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            }
        }
    }
}
