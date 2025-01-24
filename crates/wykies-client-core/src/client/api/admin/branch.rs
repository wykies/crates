use crate::Client;
use reqwest_cross::oneshot;
use wykies_shared::{
    branch::BranchDraft, const_config::path::PATH_API_ADMIN_BRANCH_CREATE, id::BranchId,
};

impl Client {
    #[tracing::instrument]
    pub fn create_branch(&self, args: &BranchDraft) -> oneshot::Receiver<anyhow::Result<BranchId>> {
        self.send_request_expect_json(PATH_API_ADMIN_BRANCH_CREATE, args)
    }
}
