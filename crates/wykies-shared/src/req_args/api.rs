use secrecy::SecretString;

pub mod admin;
#[derive(serde::Deserialize)]
pub struct ChangePasswordReqArgs {
    pub current_password: SecretString,
    pub new_password: SecretString,
    pub new_password_check: SecretString,
}
