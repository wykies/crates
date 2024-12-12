mod middleware;
mod password;

pub use middleware::validate_user_access;
pub use password::{
    argon2_settings, change_password, validate_credentials, AuthUserInfo, Credentials,
};

#[derive(Debug, Clone, Copy)]
pub struct LoginAttemptLimit(pub u8);

impl LoginAttemptLimit {
    pub fn as_i8(&self) -> i8 {
        self.0 as i8
    }
}
