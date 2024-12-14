use futures::channel::oneshot;
use secrecy::ExposeSecret as _;
use wykies_shared::{
    const_config::path::{PATH_API_CHANGE_PASSWORD, PATH_API_LOGOUT},
    req_args::api::ChangePasswordReqArgs,
};

use crate::{client::UiCallBack, Client};

pub mod admin;

impl Client {
    #[tracing::instrument(skip(args, ui_notify))]
    pub fn change_password<F>(
        &self,
        args: &ChangePasswordReqArgs,
        ui_notify: F,
    ) -> oneshot::Receiver<anyhow::Result<()>>
    where
        F: UiCallBack,
    {
        let args = serde_json::json!({
            "current_password": args.current_password.expose_secret(),
            "new_password": args.new_password.expose_secret(),
            "new_password_check": args.new_password_check.expose_secret()
        });
        self.send_request_expect_empty(PATH_API_CHANGE_PASSWORD, &args, ui_notify)
    }

    #[tracing::instrument(skip(ui_notify))]
    pub fn logout<F: UiCallBack>(&self, ui_notify: F) -> oneshot::Receiver<anyhow::Result<()>> {
        self.clear_user_info(); // Clear user info even if logout fails
        self.send_request_expect_empty(PATH_API_LOGOUT, &"", ui_notify)
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
