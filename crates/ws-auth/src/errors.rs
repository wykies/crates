use crate::WsServiceId;
use wykies_shared::host_branch::HostId;

#[derive(thiserror::Error, Debug)]
pub enum WebSocketAuthError {
    /// Client was not expected to be trying to connect
    #[error("Unexpected Client")]
    UnexpectedClient {
        client_identifier: HostId,
        ws_id: WsServiceId,
    },
    #[error("Invalid Token")]
    InvalidToken {
        client_identifier: HostId,
        ws_id: WsServiceId,
    },
    #[error("Unable to start session")]
    FailedToStartSession(anyhow::Error),
    #[error("Unexpected Error")]
    UnexpectedError(#[from] anyhow::Error),
}

#[cfg(not(target_arch = "wasm32"))]
pub mod conversions {
    use super::WebSocketAuthError;
    use actix_web::http::StatusCode;

    impl actix_web::error::ResponseError for WebSocketAuthError {
        fn status_code(&self) -> StatusCode {
            match self {
                // Using I'm a tea pot because unable to get more than the error code on the client
                // and want to give a better error message
                Self::UnexpectedClient { .. } => StatusCode::IM_A_TEAPOT,
                Self::InvalidToken { .. } => StatusCode::UNAUTHORIZED,
                Self::FailedToStartSession(_) | Self::UnexpectedError(_) => {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            }
        }
    }
}
