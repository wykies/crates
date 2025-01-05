use crate::Client;
use reqwest_cross::{oneshot, UiCallBack};
use wykies_shared::{
    branch::BranchDraft, const_config::path::PATH_API_ADMIN_BRANCH_CREATE, id::DbId,
};

impl Client {
    #[tracing::instrument(skip(ui_notify))]
    pub fn create_branch<F: UiCallBack>(
        &self,
        args: &BranchDraft,
        ui_notify: F,
    ) -> oneshot::Receiver<anyhow::Result<DbId>> {
        self.send_request_expect_json(PATH_API_ADMIN_BRANCH_CREATE, args, ui_notify)
    }
}
