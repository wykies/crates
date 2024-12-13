//! Provides authentication for web socket handlers
//!
//! Based on this article
//! <https://devcenter.heroku.com/articles/websocket-security> that cites
//! <https://lucumr.pocoo.org/2012/9/24/websockets-101/> as the original source

mod errors;
mod id;
mod manager;
mod runtime_utils;

pub use errors::WebSocketAuthError;
pub use id::{WsConnId, WsId};
pub use manager::{validate_ws_connection, AuthTokenManager};

pub use runtime_utils::{handlers, pre_screen_incoming_ws_req};
