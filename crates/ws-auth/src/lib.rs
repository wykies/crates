//! Provides authentication for web socket handlers
//!
//! Based on this article
//! <https://devcenter.heroku.com/articles/websocket-security> that cites
//! <https://lucumr.pocoo.org/2012/9/24/websockets-101/> as the original source

mod errors;
mod id;
mod manager;
mod runtime_utils;
// TODO 1: This shouldn't need to be public
pub mod handlers;

pub use errors::WebSocketAuthError;
pub use id::{WsConnId, WsId};
pub use manager::{validate_ws_connection, AuthTokenManager};

pub use runtime_utils::{
    pre_screen_incoming_ws_req, validate_connection_then_start_client_handler_loop,
};
