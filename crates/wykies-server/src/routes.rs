mod admin;
mod health_check;
mod login;
mod logout;
mod password;
mod status;

use actix_web::{HttpRequest, HttpResponse};
pub use admin::{
    branch::{branch_create, branch_list},
    host_branch::{host_branch_pair_lookup, list_host_branch_pairs, set_host_branch_pair},
    role::{role, role_assign, role_create},
    user::{list_users_and_roles, password_reset, user, user_new, user_update},
};
use anyhow::{bail, Context};
pub use health_check::health_check;
pub use login::login;
pub use logout::log_out;
pub use password::change_password;
pub use status::status;
use wykies_shared::{
    debug_panic,
    uac::{get_required_permissions, Permissions, PermissionsError},
};

pub fn execute_chained_handler<T>(
    path: &str,
    permissions: &Permissions,
    f: impl FnOnce() -> T,
) -> anyhow::Result<T> {
    let Some(required_permissions) = get_required_permissions(path) else {
        bail!("lookup of permissions for other endpoint failed");
    };
    if !permissions.includes(required_permissions) {
        return Err(PermissionsError::MissingPermissions(
            required_permissions.to_vec(),
        ))
        .context("chaining failed for permissions")?;
    }
    Ok(f())
}

#[tracing::instrument]
pub async fn not_found(req: HttpRequest) -> actix_web::Result<HttpResponse> {
    tracing::error!("Failed to match route");
    debug_panic!(format!(
        "404 - {} to '{}' Not found\n",
        req.method(),
        req.path()
    ));
    Ok(HttpResponse::NotFound().body(format!(
        "404 - {} to '{}' Not found\n",
        req.method(),
        req.path()
    )))
}
