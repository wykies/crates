use crate::{db_types::Db, errors::ConversionError, id::DbId, string_wrapper};

string_wrapper!(BranchName, 30);
string_wrapper!(BranchAddress, 200);

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct Branch {
    pub id: DbId,
    pub name: BranchName,
    pub address: BranchAddress,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct BranchDraft {
    pub name: BranchName,
    pub address: BranchAddress,
}
