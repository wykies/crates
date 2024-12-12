//! Stores settings that are not expected to need to change but grouped together
//! for discoverability and reuse. Each constant should be prefixed by the module
//! name to allow importing the constant only and still be readable

use crate::uac::RoleName;
use std::sync::LazyLock;
use wykies_time::Seconds;

pub const CHANNEL_BUFFER_SIZE: usize = 100;
pub const PANIC_ON_RARE_ERR: bool = true;

pub mod server {
    use super::*;
    /// This is possibly in addition to the graceful shutdown timeout in the
    /// API Server after the API Server closes
    pub const SERVER_SHUTDOWN_TIMEOUT: Seconds = Seconds::new(20);
    pub const DB_ACQUIRE_TIMEOUT: Seconds = Seconds::new(2);
}

pub mod client {
    use super::*;
    pub const CLIENT_IDLE_TIMEOUT: Seconds = Seconds::new(300);
    /// Using the fact that updates only happen either once a second or when
    /// there is user activity we will use this as a measure to determine when
    /// there is user activity
    pub const CLIENT_TICKS_PER_SECOND_FOR_ACTIVE: usize = 5;

    pub mod user_edit {
        use super::Seconds;

        pub const EDIT_WINDOW: Seconds = Seconds::new(60);
    }
}

pub mod error {

    use super::*;

    /// Displayed instead of crashing when a role name fails to be looked up
    pub fn err_role_name() -> &'static RoleName {
        static ERR_ROLE_NAME: LazyLock<RoleName> = LazyLock::new(|| {
            RoleName::try_from("ERROR: INVALID".to_string())
                .expect("test below ensure this is valid")
        });
        &ERR_ROLE_NAME
    }
    #[test]
    fn ensure_error_role_is_valid() {
        println!("{:?}", err_role_name());
    }
}

pub mod path {
    mod path_spec;
    pub use path_spec::PathSpec;
    pub const PATH_API_ADMIN_BRANCH_CREATE: PathSpec = PathSpec::post("/api/admin/branch/create");
    pub const PATH_API_ADMIN_HOSTBRANCH_LIST: PathSpec =
        PathSpec::get("/api/admin/host_branch/list");
    pub const PATH_API_ADMIN_HOSTBRANCH_SET: PathSpec =
        PathSpec::post("/api/admin/host_branch/set");
    pub const PATH_API_ADMIN_ROLE_ASSIGN: PathSpec = PathSpec::post("/api/admin/role/assign");
    pub const PATH_API_ADMIN_ROLE_CREATE: PathSpec = PathSpec::post("/api/admin/role/create");
    pub const PATH_API_ADMIN_ROLE: PathSpec = PathSpec::get("/api/admin/role/");
    pub const PATH_API_ADMIN_USER_NEW: PathSpec = PathSpec::post("/api/admin/user/new");
    pub const PATH_API_ADMIN_USER_PASSWORD_RESET: PathSpec =
        PathSpec::post("/api/admin/user/password_reset");
    pub const PATH_API_ADMIN_USER_UPDATE: PathSpec = PathSpec::post("/api/admin/user/update");
    pub const PATH_API_ADMIN_USER: PathSpec = PathSpec::get("/api/admin/user/");
    pub const PATH_API_ADMIN_USERS_LIST_AND_ROLES: PathSpec = PathSpec::get("/api/admin/user/list");
    pub const PATH_API_CHANGE_PASSWORD: PathSpec = PathSpec::post("/api/change_password");
    pub const PATH_API_HOSTBRANCH_LOOKUP: PathSpec = PathSpec::get("/api/host_branch/lookup");
    pub const PATH_API_LOGOUT: PathSpec = PathSpec::post("/api/logout");
    pub const PATH_BRANCHES: PathSpec = PathSpec::get("/branches");
    pub const PATH_HEALTH_CHECK: PathSpec = PathSpec::get("/health_check");
    pub const PATH_LOGIN: PathSpec = PathSpec::post("/login");
    pub const PATH_WS_PREFIX: &str = "/api/ws_token"; // All websocket requests must start with this prefix
    pub const PATH_WS_TOKEN_CHAT: PathSpec = PathSpec::post("/api/ws_token/chat");
}

pub mod web_socket {
    pub const WS_MAX_CONTINUATION_SIZE: usize = 2 * 1024 * 1024;
    pub const WS_MAX_FRAME_SIZE: usize = 128 * 1024;
}

#[cfg(test)]
mod tests {
    use static_assertions::const_assert;

    use super::web_socket::{WS_MAX_CONTINUATION_SIZE, WS_MAX_FRAME_SIZE};

    // Seems reasonable to expect 2 fames to fit before continuation frame is full
    const_assert!(WS_MAX_FRAME_SIZE * 2 <= WS_MAX_CONTINUATION_SIZE);
}
