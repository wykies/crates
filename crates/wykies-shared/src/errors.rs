use crate::{id::DbId, uac::Permission};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConversionError {
    #[error("Empty not allowed for {type_name}")]
    Empty { type_name: &'static str },
    #[error("Maximum length exceeded. {max} allowed but found {actual} for {type_name}")]
    MaxExceeded {
        max: usize,
        actual: usize,
        type_name: &'static str,
    },
}

#[derive(Debug, thiserror::Error)]
#[error("The user has not logged in")]
pub struct NotLoggedInError;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PermissionConversionError {
    #[error(
        "Only valid strings are those of length {allowed_length} but found string of length {actual_length}. Value: {value:?}"
    )]
    WrongLength {
        allowed_length: usize,
        actual_length: usize,
        value: String,
    },
    #[error("Invalid character found for {perm:?}. Only 0 or 1 expected but found {c}")]
    InvalidCharacter { c: char, perm: Permission },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DbIdConversionError {
    #[error("Negative values not supported as Id's. Value: {0}")]
    NegativeI32(i32),
    #[error("Internal value of DbId is too large for I32. Value: {0:?}")]
    TooBigForI32(DbId),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HostIdConversionError {
    #[error("No 'peer_addr' found")]
    NoPeerAddrFound,
    #[error(transparent)]
    ConvertError(#[from] ConversionError),
}

#[cfg(not(target_arch = "wasm32"))]
pub mod conversions {
    use super::*;
    use actix_web::http::StatusCode;

    impl actix_web::error::ResponseError for NotLoggedInError {
        fn status_code(&self) -> StatusCode {
            StatusCode::UNAUTHORIZED
        }
    }

    impl actix_web::error::ResponseError for ConversionError {
        fn status_code(&self) -> StatusCode {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }

    impl actix_web::error::ResponseError for PermissionConversionError {
        fn status_code(&self) -> StatusCode {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }

    impl actix_web::error::ResponseError for DbIdConversionError {
        fn status_code(&self) -> StatusCode {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
