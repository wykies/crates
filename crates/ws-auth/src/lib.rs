//! Provides authenticated access to web socket handlers

mod errors;

pub use errors::WebSocketError;

/// Distinguishes different types of Websocket services supported
#[derive(Debug, PartialEq, Eq, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct WsId(u8);

impl WsId {
    pub const fn new(value: u8) -> Self {
        Self(value)
    }
}
