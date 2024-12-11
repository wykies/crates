use thiserror::Error;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConversionError {
    #[error("Empty not allowed")]
    Empty,
    #[error("Maximum length exceeded. {max} allowed but found {actual}")]
    MaxExceeded { max: usize, actual: usize },
}

#[derive(Debug, Error)]
#[error("The user has not logged in")]
pub struct NotLoggedInError;

#[cfg(not(target_arch = "wasm32"))]
pub mod conversions {
    use super::*;
    use actix_web::http::StatusCode;

    impl actix_web::error::ResponseError for NotLoggedInError {
        fn status_code(&self) -> StatusCode {
            StatusCode::UNAUTHORIZED
        }
    }
}
