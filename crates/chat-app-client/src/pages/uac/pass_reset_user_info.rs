use wykies_client_core::Client;
use wykies_shared::{req_args::api::admin::user::PasswordResetReqArgs, uac::UserMetadata};

use crate::pages::data_state::{AwaitingType, DataState};

use super::{get_save_outcome, SaveState};

#[derive(Debug)]
pub struct PassResetUserInfo {
    pub data: PasswordResetReqArgs,
    save_status: DataState<()>,
}

impl PassResetUserInfo {
    pub(crate) fn new(user: &UserMetadata) -> Self {
        Self {
            data: PasswordResetReqArgs {
                username: user.username.clone(),
                new_password: "".to_string().into(),
            },
            save_status: Default::default(),
        }
    }

    /// Returns None if no save is ongoing
    pub fn save_outcome(&mut self) -> Option<SaveState> {
        get_save_outcome(&mut self.save_status)
    }

    pub(crate) fn save(&mut self, client_core: &Client) {
        self.save_status = DataState::AwaitingResponse(AwaitingType(
            client_core.reset_password(self.data.clone(), || {}),
        ))
    }
}
