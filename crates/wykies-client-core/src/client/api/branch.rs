use crate::Client;
use reqwest_cross::oneshot;
use wykies_shared::{
    branch::BranchDraft, branch::BranchId, const_config::path::PATH_API_BRANCH_NEW,
};

impl Client {
    #[tracing::instrument]
    pub fn branch_new(&self, args: &BranchDraft) -> oneshot::Receiver<anyhow::Result<BranchId>> {
        self.send_request_expect_json(PATH_API_BRANCH_NEW, args)
    }
}
