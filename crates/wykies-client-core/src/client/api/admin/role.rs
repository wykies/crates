use futures::channel::oneshot;
use wykies_shared::{
    const_config::path::{
        PATH_API_ADMIN_ROLE, PATH_API_ADMIN_ROLE_ASSIGN, PATH_API_ADMIN_ROLE_CREATE,
    },
    id::DbId,
    req_args::api::admin::role::{self, AssignReqArgs},
    uac::{Role, RoleDraft},
};

use crate::{client::UiCallBack, Client};

impl Client {
    #[tracing::instrument(skip(ui_notify))]
    pub fn create_role<F: UiCallBack>(
        &self,
        args: &RoleDraft,
        ui_notify: F,
    ) -> oneshot::Receiver<anyhow::Result<DbId>> {
        self.send_request_expect_json(PATH_API_ADMIN_ROLE_CREATE, args, ui_notify)
    }

    #[tracing::instrument(skip(ui_notify))]
    pub fn get_role<F: UiCallBack>(
        &self,
        role_id: DbId,
        ui_notify: F,
    ) -> oneshot::Receiver<anyhow::Result<Role>> {
        let args = role::LookupReqArgs { role_id };
        self.send_request_expect_json(PATH_API_ADMIN_ROLE, &args, ui_notify)
    }

    #[tracing::instrument(skip(ui_notify))]
    pub fn assign_role<F: UiCallBack>(
        &self,
        args: &AssignReqArgs,
        ui_notify: F,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        self.send_request_expect_empty(PATH_API_ADMIN_ROLE_ASSIGN, args, ui_notify)
    }
}
