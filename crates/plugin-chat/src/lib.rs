//! Plugin to add chat functionality
#![warn(unused_crate_dependencies)]

pub mod consts;
mod msg_types;
#[cfg(feature = "server_only")]
pub mod server_only;

pub use msg_types::{
    ChatIM, ChatImText, ChatMsg, ChatMsgsHistory, ChatUser, InitialStateBody, ReqHistoryBody,
};
