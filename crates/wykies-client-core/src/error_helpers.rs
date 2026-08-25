use reqwest_cross::{DataState, DataStateError};

/// Provides a way to use a type to store errors, helpful for reusing a type
/// that already has code associated with it for error handling
pub trait ErrorStore {
    fn set_error_state_from_anyhow<E: Into<anyhow::Error>>(&mut self, err: E);
    fn set_error_state_from_str<S: AsRef<str>>(&mut self, s: S);
}

impl<T> ErrorStore for DataState<T, anyhow::Error> {
    fn set_error_state_from_anyhow<E: Into<anyhow::Error>>(&mut self, err: E) {
        *self = Self::Failed(DataStateError::FromE(err.into()));
    }

    fn set_error_state_from_str<S: AsRef<str>>(&mut self, s: S) {
        *self = Self::Failed(DataStateError::FromE(anyhow::anyhow!("{}", s.as_ref())));
    }
}
