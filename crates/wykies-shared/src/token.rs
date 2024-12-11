use crate::random_string_def_len;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AuthToken(String);

impl AuthToken {
    pub fn new_rand() -> Self {
        random_string_def_len().into()
    }
}

impl From<String> for AuthToken {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<AuthToken> for ewebsock::WsMessage {
    fn from(value: AuthToken) -> Self {
        ewebsock::WsMessage::Text(value.0)
    }
}
