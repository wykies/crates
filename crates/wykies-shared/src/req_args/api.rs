use secrecy::SecretString;

pub mod host_branch;
pub mod role;
pub mod user;

#[derive(serde::Deserialize)]
pub struct ChangePasswordReqArgs {
    pub current_password: SecretString,
    pub new_password: SecretString,
    pub new_password_check: SecretString,
}
