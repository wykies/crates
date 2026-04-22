use egui_helpers::ScreenLockInfo;
use egui_pages::PermissionValidator;
use tracing::{debug, error, instrument, warn};
use wykies_shared::{
    const_config::client::{CLIENT_IDLE_TIMEOUT, CLIENT_TICKS_PER_SECOND_FOR_ACTIVE},
    uac::Permission,
};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct DataShared {
    pub username: String,

    #[serde(skip)]
    /// Allows for forcing password change and updating of data outside of the
    /// client-core
    is_login_completed: bool,
    #[serde(skip)]
    // TODO 4: Add option for user to change the server they are connecting to (Saving a list of
    //          recent servers)
    pub client: wykies_client_core::Client,
    #[serde(skip)]
    screen_lock_info: ScreenLockInfo,
}

impl DataShared {
    /// Doesn't do anything if the client does not have user info
    #[instrument]
    pub fn mark_login_complete(&mut self) {
        if let Some(user_info) = self.client.user_info() {
            debug!("Updating username to {}", user_info.username);
            self.username = user_info.username.clone().into();
            self.is_login_completed = true;
        } else {
            warn!("No user found in client");
        }
    }

    pub fn is_logged_in(&mut self) -> bool {
        if self.client.is_logged_in() {
            self.is_login_completed
        } else {
            self.is_login_completed = false; // Reset completed status (ensure reset after logout)
            false
        }
    }

    pub fn is_screen_locked(&mut self) -> bool {
        self.screen_lock_info.is_locked()
    }

    pub fn unlock(&mut self) {
        self.screen_lock_info.unlock()
    }

    pub fn lock(&mut self) {
        self.screen_lock_info.lock()
    }

    pub fn screen_lock_info_tick(&mut self) {
        self.screen_lock_info.tick();
    }
}

impl PermissionValidator<Permission> for DataShared {
    fn has_permissions(&self, required_permissions: &[Permission]) -> bool {
        let Some(permissions) = self.client.user_info().map(|user| user.permissions.clone()) else {
            error!(
                "Attempt to get user information when it doesn't exist. Isn't the user logged in?"
            );
            debug_assert!(
                false,
                "This shouldn't happen we should only be checking user information after login when it exists"
            );
            return false;
        };
        permissions
            .includes(required_permissions)
            .has_required_permissions()
    }
}

impl Default for DataShared {
    fn default() -> Self {
        Self {
            username: Default::default(),
            is_login_completed: Default::default(),
            client: Default::default(),
            screen_lock_info: ScreenLockInfo::new(
                CLIENT_IDLE_TIMEOUT,
                CLIENT_TICKS_PER_SECOND_FOR_ACTIVE,
            ),
        }
    }
}
