use crate::Client;
use reqwest_cross::oneshot;
use secrecy::ExposeSecret as _;
use wykies_shared::{
    const_config::path::{PATH_API_CHANGE_PASSWORD, PATH_API_LOGOUT},
    req_args::api::ChangePasswordReqArgs,
};

pub mod branch;
pub mod host_branch;
pub mod role;
pub mod user;

impl Client {
    #[tracing::instrument(skip(args))]
    pub fn change_password(
        &self,
        args: &ChangePasswordReqArgs,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        let args = serde_json::json!({
            "current_password": args.current_password.expose_secret(),
            "new_password": args.new_password.expose_secret(),
            "new_password_check": args.new_password_check.expose_secret()
        });
        self.send_request_expect_empty(PATH_API_CHANGE_PASSWORD, &args)
    }

    #[tracing::instrument]
    pub fn logout(&self) -> oneshot::Receiver<anyhow::Result<()>> {
        self.clear_user_info(); // Clear user info even if logout fails
        self.send_request_expect_empty(PATH_API_LOGOUT, &"")
    }

    #[tracing::instrument]
    pub fn logout_no_wait(&self) {
        self.clear_user_info(); // Clear user info even if logout fails
        self.send_request_no_wait(PATH_API_LOGOUT, &"");
    }

    fn clear_user_info(&self) {
        self.inner.lock().expect("mutex poisoned").user_info = None;
    }
}
