use crate::{
    id::BranchId,
    uac::{DisplayName, Permissions, Username},
};

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct UserSessionInfo {
    pub username: Username,
    pub display_name: DisplayName,
    pub branch_id: BranchId,
    pub permissions: Permissions,
}
