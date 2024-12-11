use crate::host_branch::HostId;

use super::Permission;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid User or Password")]
    InvalidUserOrPassword,
    #[error("User Locked Out")]
    LockedOut,
    #[error("User Not Enabled")]
    NotEnabled,
    #[error("Branch not set and user does not have permissions to set the branch. Client identifier '{client_identifier}'")]
    BranchNotSetAndUnableToSet { client_identifier: HostId },
    #[error("Branch not set please resend specifying desired branch to set")]
    BranchNotSetResend { client_identifier: HostId },
    #[error("Unexpected Error")]
    UnexpectedError(#[from] anyhow::Error),
}

impl AuthError {
    /// Returns `true` if the auth error is [`BranchNotSetResend`].
    ///
    /// [`BranchNotSetResend`]: AuthError::BranchNotSetResend
    #[must_use]
    pub fn is_branch_not_set_resend(&self) -> bool {
        matches!(self, Self::BranchNotSetResend { .. })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ChangePasswordError {
    // TODO 3: Add password complexity requirements
    #[error("You entered two different new passwords - the field values must match.")]
    PasswordsDoNotMatch,
    #[error("Current password validation failed: {0}")]
    CurrentPasswordWrong(#[from] AuthError),
    #[error("Unexpected Error")]
    UnexpectedError(#[from] anyhow::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum ResetPasswordError {
    #[error("You cannot reset your own password. Use change password")]
    NoResetOwnPassword,
    #[error("Current password validation failed: {0}")]
    UnexpectedError(#[from] anyhow::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum PermissionsError {
    #[error("the following permissions are missing to access this endpoint: {0:?}")]
    MissingPermissions(Vec<Permission>),
    #[error("unable to find permissions for this path '{0}'")]
    PathNotFound(String),
}

#[cfg(not(target_arch = "wasm32"))]
pub mod conversions {
    use super::*;
    use actix_web::http::StatusCode;

    impl actix_web::error::ResponseError for PermissionsError {
        fn status_code(&self) -> StatusCode {
            match self {
                PermissionsError::MissingPermissions(_) => StatusCode::FORBIDDEN,
                PermissionsError::PathNotFound(_) => StatusCode::SERVICE_UNAVAILABLE,
            }
        }
    }

    impl actix_web::error::ResponseError for AuthError {
        fn status_code(&self) -> StatusCode {
            match self {
                AuthError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
                _ => StatusCode::UNAUTHORIZED,
            }
        }
    }

    impl actix_web::error::ResponseError for ChangePasswordError {
        fn status_code(&self) -> StatusCode {
            match self {
                ChangePasswordError::PasswordsDoNotMatch
                | ChangePasswordError::CurrentPasswordWrong(_) => StatusCode::BAD_REQUEST,
                ChangePasswordError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            }
        }
    }

    impl actix_web::error::ResponseError for ResetPasswordError {
        fn status_code(&self) -> StatusCode {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
