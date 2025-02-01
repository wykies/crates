use crate::{client::DUMMY_ARGUMENT, Client};
use reqwest_cross::oneshot;
use wykies_shared::{
    branch::BranchId,
    const_config::path::{
        PATH_API_ADMIN_HOSTBRANCH_LIST, PATH_API_ADMIN_HOSTBRANCH_SET, PATH_API_HOSTBRANCH_LOOKUP,
    },
    host_branch::HostBranchPair,
    req_args::api::admin::host_branch,
};

impl Client {
    #[tracing::instrument]
    pub fn get_list_host_branch_pairs(
        &self,
    ) -> oneshot::Receiver<anyhow::Result<Vec<HostBranchPair>>> {
        self.send_request_expect_json(PATH_API_ADMIN_HOSTBRANCH_LIST, &DUMMY_ARGUMENT)
    }

    #[tracing::instrument]
    pub fn create_host_branch_pair(
        &self,
        args: &HostBranchPair,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        self.send_request_expect_empty(PATH_API_ADMIN_HOSTBRANCH_SET, args)
    }

    #[tracing::instrument]
    pub fn get_host_branch_pair(
        &self,
        args: &host_branch::LookupReqArgs,
    ) -> oneshot::Receiver<anyhow::Result<Option<BranchId>>> {
        self.send_request_expect_json(PATH_API_HOSTBRANCH_LOOKUP, args)
    }
}
