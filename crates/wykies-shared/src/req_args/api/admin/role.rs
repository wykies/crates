use crate::{id::DbId, uac::Username};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct AssignReqArgs {
    pub username: Username,
    pub role_id: DbId,
}
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LookupReqArgs {
    pub role_id: DbId,
}
