//! Shared items related to user account control

mod errors;
mod passwords;
mod permissions;
mod responses;
mod role;
mod user;

pub use errors::{AuthError, ChangePasswordError, PermissionsError, ResetPasswordError};
pub use passwords::{PasswordComplexity, PasswordComplexityError};
pub use permissions::{
    default_permissions, get_required_permissions, init_permissions_to_defaults,
    try_set_permissions, Permission, PermissionCheckOutcome, PermissionMap, Permissions,
};
pub use responses::LoginResponse;
pub use role::{Role, RoleDescription, RoleDraft, RoleId, RoleIdAndName, RoleName};
pub use user::{DisplayName, ListUsersRoles, UserInfo, UserMetadata, UserMetadataDiff, Username};
