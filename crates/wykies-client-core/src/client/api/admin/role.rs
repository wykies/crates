use crate::Client;
use reqwest_cross::oneshot;
use wykies_shared::{
    const_config::path::{
        PATH_API_ADMIN_ROLE, PATH_API_ADMIN_ROLE_ASSIGN, PATH_API_ADMIN_ROLE_CREATE,
    },
    req_args::api::admin::role::{self, AssignReqArgs},
    uac::RoleId,
    uac::{Role, RoleDraft},
};

impl Client {
    #[tracing::instrument]
    pub fn create_role(&self, args: &RoleDraft) -> oneshot::Receiver<anyhow::Result<RoleId>> {
        self.send_request_expect_json(PATH_API_ADMIN_ROLE_CREATE, args)
    }

    #[tracing::instrument]
    pub fn get_role(&self, role_id: RoleId) -> oneshot::Receiver<anyhow::Result<Role>> {
        let args = role::LookupReqArgs { role_id };
        self.send_request_expect_json(PATH_API_ADMIN_ROLE, &args)
    }

    #[tracing::instrument]
    pub fn assign_role(&self, args: &AssignReqArgs) -> oneshot::Receiver<anyhow::Result<()>> {
        self.send_request_expect_empty(PATH_API_ADMIN_ROLE_ASSIGN, args)
    }
}
