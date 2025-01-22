use std::fmt::{Debug, Display};
use uuid::Uuid;

#[derive(
    Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct WSConnId(Uuid);

pub struct WSConnTxRx {
    pub tx: ewebsock::WsSender,
    pub rx: ewebsock::WsReceiver,
}

impl WSConnId {
    pub fn new_rand() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Display for WSConnId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Debug for WSConnTxRx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebSocketConnection {{ tx, rx }} ")
    }
}
