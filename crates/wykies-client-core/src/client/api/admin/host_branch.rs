use futures::channel::oneshot;
use wykies_shared::{
    const_config::path::{
        PATH_API_ADMIN_HOSTBRANCH_LIST, PATH_API_ADMIN_HOSTBRANCH_SET, PATH_API_HOSTBRANCH_LOOKUP,
    },
    host_branch::HostBranchPair,
    id::DbId,
    req_args::api::admin::host_branch,
};

use crate::{
    client::{UiCallBack, DUMMY_ARGUMENT},
    Client,
};

impl Client {
    #[tracing::instrument(skip(ui_notify))]
    pub fn get_list_host_branch_pairs<F: UiCallBack>(
        &self,
        ui_notify: F,
    ) -> oneshot::Receiver<anyhow::Result<Vec<HostBranchPair>>> {
        self.send_request_expect_json(PATH_API_ADMIN_HOSTBRANCH_LIST, &DUMMY_ARGUMENT, ui_notify)
    }

    #[tracing::instrument(skip(ui_notify))]
    pub fn create_host_branch_pair<F: UiCallBack>(
        &self,
        args: &HostBranchPair,
        ui_notify: F,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        self.send_request_expect_empty(PATH_API_ADMIN_HOSTBRANCH_SET, args, ui_notify)
    }

    #[tracing::instrument(skip(ui_notify))]
    pub fn get_host_branch_pair<F: UiCallBack>(
        &self,
        args: &host_branch::LookupReqArgs,
        ui_notify: F,
    ) -> oneshot::Receiver<anyhow::Result<Option<DbId>>> {
        self.send_request_expect_json(PATH_API_HOSTBRANCH_LOOKUP, args, ui_notify)
    }
}
