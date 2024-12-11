use crate::host_branch::HostId;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LookupReqArgs {
    pub host_id: HostId,
}
