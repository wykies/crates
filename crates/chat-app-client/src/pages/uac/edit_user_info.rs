use super::{get_save_outcome, SaveState};
use anyhow::anyhow;
use reqwest_cross::{Awaiting, DataState};
use wykies_client_core::Client;
use wykies_shared::internal_error;
use wykies_shared::{
    const_config::client::user_edit,
    uac::{UserMetadata, UserMetadataDiff},
};
use wykies_time::{Seconds, Timestamp};

#[derive(Debug)]
pub struct EditUserInfo {
    original_user: UserMetadata,
    edit_user: DataState<UserMetadata>,
    load_time: Option<Timestamp>,
    save_status: DataState<()>,
}

impl EditUserInfo {
    pub fn new(original_user: UserMetadata) -> Self {
        Self {
            original_user,
            edit_user: Default::default(),
            load_time: Default::default(),
            save_status: Default::default(),
        }
    }

    pub fn original_user(&self) -> &UserMetadata {
        &self.original_user
    }

    /// Returns if the edited user is different from the original one
    pub fn has_changes(&self) -> bool {
        let DataState::Present(edit_user) = self.edit_user.as_ref() else {
            return false;
        };
        edit_user != &self.original_user
    }

    /// If there is an edit user available it returns a immutable copy of the
    /// original user and a mutable copy of the edit user
    ///
    /// If the edit user is not available and load has started it polls for
    /// updates
    pub fn org_user_and_edit_user(
        &mut self,
        ui: &mut egui::Ui,
        client_core: &Client,
    ) -> Option<(&UserMetadata, &mut UserMetadata)> {
        self.check_edit_user_valid();
        if self.has_load_started() {
            // Only load if loading already started
            self.load_user_info(ui, client_core);
        }
        if let DataState::Present(edit_user) = self.edit_user.as_mut() {
            Some((&self.original_user, edit_user))
        } else {
            None
        }
    }

    /// Returns true if the load has been started (includes true if load already
    /// completed)
    pub fn has_load_started(&self) -> bool {
        !self.edit_user.is_none()
    }

    /// Starts user loading if not started and on subsequent calls polls the
    /// returned awaiting start or does nothing if loading completed
    pub fn load_user_info(&mut self, ui: &mut egui::Ui, client_core: &Client) {
        if !self.edit_user.is_present() {
            self.load_time = Some(Timestamp::now());
            self.edit_user.egui_get(ui, Some("Clear Error"), || {
                Awaiting(client_core.get_user(self.original_user.username.clone(), || {}))
            });
        }
    }

    pub fn unload_user_info(&mut self) {
        self.load_time = None;
        self.edit_user = DataState::None;
    }
    pub fn time_before_auto_unload_user(&mut self) -> Option<Seconds> {
        let timestamp = self.load_time?;
        let elapsed = timestamp.elapsed().unwrap_or_else(|| {
            internal_error!(format!("timestamp in future: {timestamp:?}"));
            0u64.into()
        });
        Some(user_edit::EDIT_WINDOW.saturating_sub(elapsed))
    }

    fn check_edit_user_valid(&mut self) {
        if let Some(secs) = self.time_before_auto_unload_user() {
            if secs.is_zero() {
                self.unload_user_info();
            }
        }
    }

    /// Initiates the save of edits to the database
    ///
    /// NOTE: Expects to only be called if there are changes
    pub fn save(&mut self, ui: &mut egui::Ui, client_core: &Client) {
        let Some((org_user, edit_user)) = self.org_user_and_edit_user(ui, client_core) else {
            self.save_status = DataState::Failed("Failed to get user edits".into());
            return;
        };

        let diff = match UserMetadataDiff::from_diff(org_user, edit_user) {
            Ok(opt) => match opt {
                Some(diff) => diff,
                None => {
                    self.save_status =
                        DataState::Failed(anyhow!(internal_error!("No changes found")).into());
                    return;
                }
            },
            Err(e) => {
                self.save_status = DataState::Failed(internal_error!(e.to_string()).into());
                return;
            }
        };
        self.save_status =
            DataState::AwaitingResponse(Awaiting(client_core.update_user(diff, || {})));
    }

    /// Returns None if no save is ongoing
    pub fn save_outcome(&mut self) -> Option<SaveState> {
        get_save_outcome(&mut self.save_status)
    }
}
