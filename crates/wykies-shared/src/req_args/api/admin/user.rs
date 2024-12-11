use secrecy::SecretString;

use crate::{
    id::DbId,
    uac::{DisplayName, Username},
};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LookupReqArgs {
    pub username: Username,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct NewUserReqArgs {
    pub username: Username,
    pub display_name: DisplayName,
    pub password: SecretString,
    pub assigned_role: Option<DbId>,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct PasswordResetReqArgs {
    pub username: Username,
    pub new_password: SecretString,
}
