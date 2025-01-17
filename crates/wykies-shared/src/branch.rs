#[cfg(feature = "server_only")]
use crate::db_types::Db;
use crate::{errors::ConversionError, id::DbId, string_wrapper, AlwaysCase};

string_wrapper!(BranchName, 30, AlwaysCase::Any);

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct Branch {
    pub id: DbId,
    pub name: BranchName,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct BranchDraft {
    pub name: BranchName,
}
