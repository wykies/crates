use futures::channel::oneshot;
use secrecy::ExposeSecret;
use wykies_shared::{
    const_config::path::{
        PATH_API_ADMIN_USER, PATH_API_ADMIN_USERS_LIST_AND_ROLES, PATH_API_ADMIN_USER_NEW,
        PATH_API_ADMIN_USER_PASSWORD_RESET, PATH_API_ADMIN_USER_UPDATE,
    },
    req_args::{
        api::admin::user::{self, NewUserReqArgs, PasswordResetReqArgs},
        RonWrapper,
    },
    uac::{ListUsersRoles, UserMetadata, UserMetadataDiff, Username},
};

use crate::{
    client::{UiCallBack, DUMMY_ARGUMENT},
    Client,
};

impl Client {
    #[tracing::instrument(skip(ui_notify))]
    pub fn get_user<F: UiCallBack>(
        &self,
        username: Username,
        ui_notify: F,
    ) -> oneshot::Receiver<anyhow::Result<UserMetadata>> {
        let args = user::LookupReqArgs { username };
        self.send_request_expect_json(PATH_API_ADMIN_USER, &args, ui_notify)
    }

    #[tracing::instrument(skip(ui_notify))]
    pub fn new_user<F: UiCallBack>(
        &self,
        user: NewUserReqArgs,
        ui_notify: F,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        let args = serde_json::json!({
            "username": user.username,
            "display_name": user.display_name,
            "password": user.password.expose_secret(),
            "assigned_role": user.assigned_role
        });
        self.send_request_expect_empty(PATH_API_ADMIN_USER_NEW, &args, ui_notify)
    }

    #[tracing::instrument(skip(ui_notify))]
    pub fn reset_password<F: UiCallBack>(
        &self,
        args: PasswordResetReqArgs,
        ui_notify: F,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        let args = serde_json::json!({
            "username": args.username,
            "new_password": args.new_password.expose_secret()
        });
        self.send_request_expect_empty(PATH_API_ADMIN_USER_PASSWORD_RESET, &args, ui_notify)
    }

    #[tracing::instrument(skip(ui_notify))]
    pub fn update_user<F: UiCallBack>(
        &self,
        diff: UserMetadataDiff,
        ui_notify: F,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        let wrapped = RonWrapper::new(&diff).expect("failed to create ron wrapper");
        self.send_request_expect_empty(PATH_API_ADMIN_USER_UPDATE, &wrapped, ui_notify)
    }

    #[tracing::instrument(skip(ui_notify))]
    pub fn list_users_and_roles<F: UiCallBack>(
        &self,
        ui_notify: F,
    ) -> oneshot::Receiver<anyhow::Result<ListUsersRoles>> {
        self.send_request_expect_json(
            PATH_API_ADMIN_USERS_LIST_AND_ROLES,
            &DUMMY_ARGUMENT,
            ui_notify,
        )
    }
}
