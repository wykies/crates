use crate::uac::RoleId;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LookupReqArgs {
    pub role_id: RoleId,
}
