#[cfg(feature = "server_only")]
use crate::db_types::Db;
use crate::{
    char_array_wrapper, errors::ConversionError, id::BranchId, string_wrapper, AlwaysCase,
};

string_wrapper!(BranchName, 30, AlwaysCase::Any);
char_array_wrapper!(BranchShortName, 2, AlwaysCase::Upper);

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct Branch {
    pub id: BranchId,
    pub name: BranchName,
    pub short_name: BranchShortName,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct BranchDraft {
    pub name: BranchName,
    pub short_name: BranchShortName,
}
