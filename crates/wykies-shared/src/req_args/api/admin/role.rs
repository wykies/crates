use crate::{id::RoleId, uac::Username};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct AssignReqArgs {
    pub username: Username,
    pub role_id: RoleId,
}
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LookupReqArgs {
    pub role_id: RoleId,
}
