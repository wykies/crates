use anyhow::Context;
use std::fmt::Display;
#[cfg(feature = "server_only")]
use wykies_shared::db_types::Db;
use wykies_shared::{errors::ConversionError, string_wrapper, uac::Username, AlwaysCase};
use wykies_time::Timestamp;

string_wrapper!(ChatImText, 255, AlwaysCase::Any);

impl TryFrom<Vec<u8>> for ChatImText {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        String::from_utf8(value)
            .context("failed to convert byte array into String")?
            .trim_matches(char::from(0))
            .try_into()
            .context("failed to convert string into ChatImText")
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
/// Messages sent between the server and client
pub enum ChatMsg {
    UserJoined(ChatUser),
    UserLeft(ChatUser),
    IM(ChatIM),
    InitialState(InitialStateBody),
    ReqHistory(ReqHistoryBody),
    RespHistory(RespHistoryBody),
}

#[derive(
    Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct ChatUser(Username);

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct ChatIM {
    pub author: Username,
    pub timestamp: Timestamp,
    pub content: ChatImText,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct InitialStateBody {
    /// The users connected right now including their multiplicity (saturates at
    /// 256)
    pub connected_users: Vec<(ChatUser, u8)>,
    pub history: RespHistoryBody,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct ReqHistoryBody {
    pub qty: u8,
    /// The latest timestamp allowed in the response (Might duplicate the some
    /// transactions but shouldn't be many. The client is responsible to
    /// deduplicate)
    pub latest_timestamp: Timestamp,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct RespHistoryBody {
    pub ims: Vec<ChatIM>,
}

impl ChatUser {
    pub fn new(value: Username) -> Self {
        Self(value)
    }
}

impl From<ChatUser> for Username {
    fn from(value: ChatUser) -> Self {
        value.0
    }
}

impl Display for ChatIM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let time = self.timestamp.as_local_datetime().format("%T");
        let author = self.author.to_string();
        let msg = self.content.to_string();
        write!(f, "{time} {author}: {msg}")
    }
}

impl Display for ChatUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
