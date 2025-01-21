use anyhow::{bail, Context};
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
    RespHistory(ChatMsgsHistory),
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
    pub history: ChatMsgsHistory,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct ReqHistoryBody {
    /// We use a u8 so no matter what value the client sets it will always be
    /// reasonable
    pub qty: u8,
    /// The latest timestamp allowed in the response (Might duplicate the some
    /// transactions but shouldn't be many. The client is responsible to
    /// deduplicate)
    pub latest_timestamp: Timestamp,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct ChatMsgsHistory {
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

impl ChatMsgsHistory {
    pub fn push(&mut self, im: ChatIM) {
        self.ims.push(im);
    }

    /// Merges two history objects under the assumption that the messages in
    /// `other` are older, returning an error if that is not the case
    pub fn prepend_other(&mut self, mut other: Self) -> anyhow::Result<()> {
        // Sort lists to ensure ordered by timestamp
        self.sort_by_timestamp();
        other.sort_by_timestamp();

        // Ensure the precondition about the objects is met
        match (self.first(), other.last()) {
            (Some(first), Some(last)) if first.timestamp < last.timestamp => {
                bail!("Prepending Chat IM history failed. Last message in other is after our first message. Our first: {first:?}, Other Last: {last:?}");
            }
            _ => (), // No possible conflict in any other case
        }

        // Remove any duplicates from the end of end of what was returned.
        let possibly_duplicated_timestamp = self.first().map(|x| x.timestamp);
        if let Some(possibly_duplicated_timestamp) = possibly_duplicated_timestamp {
            // Get range of possibly duplicated values
            let mut last_index_with_same_timestamp = 0;
            for (i, im) in self.ims.iter().enumerate() {
                if im.timestamp == possibly_duplicated_timestamp {
                    last_index_with_same_timestamp = i;
                } else {
                    // No more can match because we sorted the list
                    break;
                }
            }
            let range_to_consider = &self.ims[..=last_index_with_same_timestamp];

            // Only keep non-duplicated values
            other.ims.retain(|x| {
                x.timestamp != possibly_duplicated_timestamp || !range_to_consider.contains(x)
            });
        }

        other.ims.append(&mut self.ims);

        // Keep other history now that we've added all our messages to it
        std::mem::swap(&mut self.ims, &mut other.ims);

        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = &ChatIM> {
        self.ims.iter()
    }

    pub fn first(&self) -> Option<&ChatIM> {
        self.ims.first()
    }
    pub fn last(&self) -> Option<&ChatIM> {
        self.ims.last()
    }

    fn sort_by_timestamp(&mut self) {
        self.ims.sort_by_key(|x| x.timestamp);
    }

    pub fn earliest_timestamp_or_now(&self) -> Timestamp {
        self.first()
            .map(|chat_im| chat_im.timestamp)
            .unwrap_or_else(Timestamp::now)
    }

    pub fn len(&self) -> usize {
        self.ims.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
