//! Plugin to add chat functionality

pub mod consts;
mod msg_types;
#[cfg(feature = "server_only")]
pub mod server_only;

pub use msg_types::{
    ChatIM, ChatImText, ChatMsg, ChatUser, InitialStateBody, ReqHistoryBody, RespHistoryBody,
};
