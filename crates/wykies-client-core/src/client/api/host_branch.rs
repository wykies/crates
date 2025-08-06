use crate::{Client, client::DUMMY_ARGUMENT};
use reqwest_cross::oneshot;
use wykies_shared::{
    branch::BranchId,
    const_config::path::{PATH_API_HOSTBRANCH, PATH_API_HOSTBRANCH_LIST, PATH_API_HOSTBRANCH_SET},
    host_branch::HostBranchPair,
    req_args::api::host_branch,
};

impl Client {
    #[tracing::instrument]
    pub fn host_branch_pair_list(&self) -> oneshot::Receiver<anyhow::Result<Vec<HostBranchPair>>> {
        self.send_request_expect_json(PATH_API_HOSTBRANCH_LIST, &DUMMY_ARGUMENT)
    }

    #[tracing::instrument]
    pub fn host_branch_pair_set(
        &self,
        args: &HostBranchPair,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        self.send_request_expect_empty(PATH_API_HOSTBRANCH_SET, args)
    }

    #[tracing::instrument]
    pub fn host_branch_pair_get(
        &self,
        args: &host_branch::LookupReqArgs,
    ) -> oneshot::Receiver<anyhow::Result<Option<BranchId>>> {
        self.send_request_expect_json(PATH_API_HOSTBRANCH, args)
    }
}
