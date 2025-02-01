use crate::uac::{DisplayName, RoleId, Username};
use secrecy::SecretString;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LookupReqArgs {
    pub username: Username,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct NewUserReqArgs {
    pub username: Username,
    pub display_name: DisplayName,
    pub password: SecretString,
    pub assigned_role: Option<RoleId>,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct PasswordResetReqArgs {
    pub username: Username,
    pub new_password: SecretString,
}
