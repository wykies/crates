use crate::{client::DUMMY_ARGUMENT, Client};
use reqwest_cross::oneshot;
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

impl Client {
    #[tracing::instrument]
    pub fn get_user(&self, username: Username) -> oneshot::Receiver<anyhow::Result<UserMetadata>> {
        let args = user::LookupReqArgs { username };
        self.send_request_expect_json(PATH_API_ADMIN_USER, &args)
    }

    #[tracing::instrument]
    pub fn new_user(&self, user: NewUserReqArgs) -> oneshot::Receiver<anyhow::Result<()>> {
        let args = serde_json::json!({
            "username": user.username,
            "display_name": user.display_name,
            "password": user.password.expose_secret(),
            "assigned_role": user.assigned_role
        });
        self.send_request_expect_empty(PATH_API_ADMIN_USER_NEW, &args)
    }

    #[tracing::instrument]
    pub fn reset_password(
        &self,
        args: PasswordResetReqArgs,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        let args = serde_json::json!({
            "username": args.username,
            "new_password": args.new_password.expose_secret()
        });
        self.send_request_expect_empty(PATH_API_ADMIN_USER_PASSWORD_RESET, &args)
    }

    #[tracing::instrument]
    pub fn update_user(&self, diff: UserMetadataDiff) -> oneshot::Receiver<anyhow::Result<()>> {
        let wrapped = RonWrapper::new(&diff).expect("failed to create ron wrapper");
        self.send_request_expect_empty(PATH_API_ADMIN_USER_UPDATE, &wrapped)
    }

    #[tracing::instrument]
    pub fn list_users_and_roles(&self) -> oneshot::Receiver<anyhow::Result<ListUsersRoles>> {
        self.send_request_expect_json(PATH_API_ADMIN_USERS_LIST_AND_ROLES, &DUMMY_ARGUMENT)
    }
}
