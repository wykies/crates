use anyhow::Context;
use reqwest_cross::{Awaiting, DataState};
use secrecy::SecretString;
use wykies_client_core::Client;
use wykies_shared::{id::DbId, req_args::api::admin::user::NewUserReqArgs};

use super::SaveState;

#[derive(Debug)]
pub struct NewUserInfo {
    pub username: String,
    pub display_name: String,
    pub password: SecretString,
    pub assigned_role: Option<DbId>,
    save_status: DataState<()>,
}

impl NewUserInfo {
    pub fn new() -> Self {
        Self {
            username: Default::default(),
            display_name: Default::default(),
            password: "".to_string().into(),
            assigned_role: Default::default(),
            save_status: Default::default(),
        }
    }

    /// Returns None if no save is ongoing
    pub fn save_outcome(&mut self) -> Option<SaveState> {
        match self.save_status.as_mut() {
            DataState::None => {
                // No action no save ongoing
                None
            }
            DataState::AwaitingResponse(rx) => {
                if let Some(new_state) = DataState::await_data(rx) {
                    self.save_status = new_state;
                }
                Some(SaveState::Ongoing)
            }
            DataState::Present(_data) => Some(SaveState::Completed),
            DataState::Failed(e) => Some(SaveState::Failed(format!("Save failed. {e}"))),
        }
    }

    /// Initiates the save of edits to the database
    pub fn save(&mut self, client_core: &Client) {
        match self.try_into_req_args() {
            Ok(req_args) => {
                self.save_status =
                    DataState::AwaitingResponse(Awaiting(client_core.new_user(req_args, || {})))
            }
            Err(e) => self.save_status = DataState::Failed(e.into()),
        }
    }

    fn try_into_req_args(&self) -> anyhow::Result<NewUserReqArgs> {
        let username = self
            .username
            .clone()
            .try_into()
            .context("invalid username")?;
        let display_name = self
            .display_name
            .clone()
            .try_into()
            .context("invalid display name")?;
        let password = self.password.clone();
        let assigned_role = self.assigned_role;

        Ok(NewUserReqArgs {
            username,
            display_name,
            password,
            assigned_role,
        })
    }
}
