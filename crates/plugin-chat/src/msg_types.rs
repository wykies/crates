use anyhow::Context;
use std::{fmt::Display, marker::PhantomData};
use wykies_shared::{errors::ConversionError, uac::Username};
use wykies_time::Timestamp;

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
pub struct ChatImText(String);

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct InitialStateBody {
    /// The users connected right now including their multiplicity (saturates at
    /// 256)
    pub connected_users: Vec<(ChatUser, u8)>,
    pub history: RespHistoryBody,
    #[serde(skip)]
    server_only: PhantomData<()>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct ReqHistoryBody {
    pub qty: u8,
    /// The latest timestamp allowed in the response (Might duplicate the some
    /// transactions but shouldn't be many. The client is responsible to
    /// deduplicate)
    pub latest_timestamp: Timestamp,
    #[serde(skip)]
    client_only: PhantomData<()>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct RespHistoryBody {
    pub ims: Vec<ChatIM>,
    #[serde(skip)]
    server_only: PhantomData<()>,
}

impl RespHistoryBody {
    #[cfg(feature = "server_only")]
    pub fn new(ims: Vec<ChatIM>) -> Self {
        Self {
            ims,
            server_only: PhantomData,
        }
    }
}

impl ReqHistoryBody {
    #[cfg(feature = "client_only")]
    pub fn new(qty: u8, latest_timestamp: Timestamp) -> Self {
        Self {
            qty,
            latest_timestamp,
            client_only: PhantomData,
        }
    }
}

impl InitialStateBody {
    #[cfg(feature = "server_only")]
    pub fn new(connected_users: Vec<(ChatUser, u8)>, history: RespHistoryBody) -> Self {
        Self {
            connected_users,
            history,
            server_only: PhantomData,
        }
    }
}

impl ChatUser {
    #[cfg(feature = "server_only")] // Only allow server to create this type
    pub fn new(value: Username) -> Self {
        Self(value)
    }
}

impl From<ChatImText> for String {
    fn from(value: ChatImText) -> Self {
        value.0
    }
}

impl From<ChatUser> for Username {
    fn from(value: ChatUser) -> Self {
        value.0
    }
}

impl ChatImText {
    pub const MAX_LENGTH: usize = 255;
}

impl TryFrom<String> for ChatImText {
    type Error = ConversionError;

    fn try_from(mut value: String) -> Result<Self, Self::Error> {
        if value.len() != value.trim().len() {
            value = value.trim().to_string();
        }
        if value.len() > Self::MAX_LENGTH {
            return Err(ConversionError::MaxExceeded {
                max: Self::MAX_LENGTH,
                actual: value.len(),
            });
        }
        Ok(Self(value))
    }
}

impl TryFrom<&str> for ChatImText {
    type Error = ConversionError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.to_string().try_into()
    }
}

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

impl AsRef<str> for ChatImText {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for ChatImText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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
