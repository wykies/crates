use std::collections::BTreeMap;

use anyhow::{bail, Ok};
use plugin_chat::ChatUser;

#[derive(Debug, Default)]
pub struct ConnectedUsers {
    users: BTreeMap<ChatUser, u8>,
}

impl ConnectedUsers {
    pub fn user_joined(&mut self, user: ChatUser) {
        let qty = self.users.entry(user).or_default();
        *qty = qty.saturating_add(1);
    }

    pub fn user_left(&mut self, user: ChatUser) -> anyhow::Result<()> {
        match self.users.get_mut(&user) {
            Some(qty) => {
                *qty = qty.saturating_sub(1);
                if *qty == 0 {
                    self.users.remove(&user);
                }
                Ok(())
            }
            None => bail!("user ({user:?}) not found"),
        }
    }

    pub fn merge_initial_users(&mut self, users: Vec<(ChatUser, u8)>) {
        for (user, new_qty) in users {
            let qty = self.users.entry(user).or_default();
            *qty = qty.saturating_add(new_qty);
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&ChatUser, &u8)> {
        self.users.iter()
    }
}
