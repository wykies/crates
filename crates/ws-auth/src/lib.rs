//! Provides authentication for web socket handlers
//!
//! Based on this article
//! <https://devcenter.heroku.com/articles/websocket-security> that cites
//! <https://lucumr.pocoo.org/2012/9/24/websockets-101/> as the original source

#![warn(unused_crate_dependencies)]

mod errors;
mod handlers;
mod id;
mod manager;
mod runtime_utils;
mod traits;

pub use errors::WebSocketAuthError;
pub use handlers::ws_get_route_add_closures;
pub use id::{WsConnId, WsId};
pub use manager::{validate_ws_connection, AuthTokenManager};
pub use traits::ClientLoopController;
