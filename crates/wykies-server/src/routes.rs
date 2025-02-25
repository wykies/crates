mod branch;
mod health_check;
mod host_branch;
mod login;
mod logout;
mod password;
mod role;
mod status;
mod user;

use actix_web::{HttpRequest, HttpResponse};
use anyhow::Context;
pub use branch::{branch_list, branch_new};
pub use health_check::health_check;
pub use host_branch::{host_branch_pair_list, host_branch_pair_lookup, host_branch_pair_set};
pub use login::login;
pub use logout::log_out;
pub use password::change_password;
pub use role::{role, role_new};
pub use status::status;
pub use user::{password_reset, role_assign, user, user_new, user_update, users_and_roles_list};
use wykies_shared::{debug_panic, uac::Permissions};

pub fn execute_chained_handler<T>(
    path: &str,
    permissions: &Permissions,
    f: impl FnOnce() -> T,
) -> anyhow::Result<T> {
    permissions
        .is_allowed_access(path)
        .context("unable to determine permissions needed for other endpoint")?
        .converting_missing_perms_to_error()
        .context("permissions not held to access other endpoint")?;
    Ok(f())
}

#[tracing::instrument]
pub async fn not_found(req: HttpRequest) -> actix_web::Result<HttpResponse> {
    tracing::error!("Failed to match route");
    debug_panic!("404 - {} to '{}' Not found\n", req.method(), req.path());
    Ok(HttpResponse::NotFound().body(format!(
        "404 - {} to '{}' Not found\n",
        req.method(),
        req.path()
    )))
}
