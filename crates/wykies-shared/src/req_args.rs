//! This module stores the expected format of the arguments for the requests
//! The structure of the module is supposed to match the path of the endpoints.
//! For example `/api/change_password` would map to
//! [`api::ChangePasswordReqArgs`] Some structs are not serializable but are
//! still here to included here to know what needs to be sent

use crate::id::DbId;
use anyhow::Context;
use secrecy::{ExposeSecret, SecretString};
use std::fmt::Debug;

pub mod api;

/// This struct exits because serde_json cannot round trip all types
///
/// Specifically the problem the we ran into was not being able to do
/// Option<Option<T>> https://github.com/serde-rs/json/issues/1096
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RonWrapper {
    data_as_ron_str: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct LoginReqArgs {
    // TODO 5: Is there a downside to making this a Username type instead of String
    pub username: String,
    pub password: SecretString,
    /// Provides a way to choose what branch to set if no branch is already
    /// saved in the database. Will be ignored if the branch is already set
    /// in the database. Will also not be used even if it's needed and the
    /// user doesn't have the required permissions.
    pub branch_to_set: Option<DbId>,
}

impl LoginReqArgs {
    pub fn new<S: Into<String>>(username: S, password: SecretString) -> Self {
        Self {
            username: username.into(),
            password,
            branch_to_set: None,
        }
    }

    pub fn new_with_branch(username: String, password: SecretString, branch_to_set: DbId) -> Self {
        Self {
            username,
            password,
            branch_to_set: Some(branch_to_set),
        }
    }

    pub fn username(mut self, username: String) -> Self {
        self.username = username;
        self
    }

    pub fn password(mut self, password: SecretString) -> Self {
        self.password = password;
        self
    }

    pub fn branch_to_set(mut self, branch_to_set: Option<DbId>) -> Self {
        self.branch_to_set = branch_to_set;
        self
    }
}

impl Debug for LoginReqArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginReqArgs")
            .field("username", &self.username)
            .field("has_password", &!self.password.expose_secret().is_empty())
            .field("branch_to_set", &self.branch_to_set)
            .finish()
    }
}

impl RonWrapper {
    pub fn new<T: serde::Serialize>(data: &T) -> anyhow::Result<Self> {
        Ok(Self {
            data_as_ron_str: ron::to_string(data).context("failed to serialize to ron")?,
        })
    }

    pub fn deserialize<'a, T>(&'a self) -> anyhow::Result<T>
    where
        T: serde::de::Deserialize<'a>,
    {
        ron::from_str(&self.data_as_ron_str).context("failed to deserialize from ron")
    }
}
