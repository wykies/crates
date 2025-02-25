use crate::Client;
use reqwest_cross::oneshot;
use wykies_shared::{
    const_config::path::{PATH_API_ROLE, PATH_API_ROLE_NEW, PATH_API_USER_ROLE_SET},
    req_args::api::{role, user::AssignReqArgs},
    uac::{Role, RoleDraft, RoleId},
};

impl Client {
    #[tracing::instrument]
    pub fn role_new(&self, args: &RoleDraft) -> oneshot::Receiver<anyhow::Result<RoleId>> {
        self.send_request_expect_json(PATH_API_ROLE_NEW, args)
    }

    #[tracing::instrument]
    pub fn role_get(&self, role_id: RoleId) -> oneshot::Receiver<anyhow::Result<Role>> {
        let args = role::LookupReqArgs { role_id };
        self.send_request_expect_json(PATH_API_ROLE, &args)
    }

    #[tracing::instrument]
    pub fn assign_role(&self, args: &AssignReqArgs) -> oneshot::Receiver<anyhow::Result<()>> {
        self.send_request_expect_empty(PATH_API_USER_ROLE_SET, args)
    }
}
