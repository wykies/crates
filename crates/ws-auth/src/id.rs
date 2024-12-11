use rand::Rng as _;
use std::{hash::Hash, sync::Arc};
use wykies_shared::session::UserSessionInfo;

/// Distinguishes different types of Websocket services supported
#[derive(Debug, PartialEq, Eq, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct WsId(u8);

impl WsId {
    #[cfg(test)]
    pub(crate) const TEST1: Self = Self::new(1);

    pub const fn new(value: u8) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone)]
/// Websocket Connection ID
/// Includes user session info as this is not available in the websocket context
/// Only the id is used for hashing and equality checks
pub struct WsConnId {
    id: usize,
    pub user_info: Arc<UserSessionInfo>,
}

impl WsConnId {
    pub fn new(user_info: Arc<UserSessionInfo>) -> Self {
        Self {
            id: rand::thread_rng().gen::<usize>(),
            user_info,
        }
    }
}

impl PartialEq for WsConnId {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for WsConnId {}

impl Hash for WsConnId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
