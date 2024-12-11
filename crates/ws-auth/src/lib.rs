//! Provides authentication for web socket handlers

mod errors;
mod id;
mod manager;

pub use errors::WebSocketAuthError;
pub use id::{WsConnId, WsId};
pub use manager::{validate_ws_connection, AuthTokenManager};
