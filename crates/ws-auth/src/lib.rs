//! Provides authenticated access to web socket handlers

mod errors;
mod manager;

pub use errors::WebSocketError;
pub use manager::{validate_ws_connection, AuthTokenManager};

/// Distinguishes different types of Websocket services supported
#[derive(Debug, PartialEq, Eq, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct WsId(u8);

impl WsId {
    #[cfg(test)]
    const TEST1: Self = Self::new(1);

    pub const fn new(value: u8) -> Self {
        Self(value)
    }
}
