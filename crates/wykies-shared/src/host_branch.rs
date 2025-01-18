#[cfg(feature = "server_only")]
use crate::db_types::Db;
use crate::{errors::ConversionError, id::DbId, string_wrapper, AlwaysCase};

string_wrapper!(HostId, 50, AlwaysCase::Any);

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct HostBranchPair {
    pub host_id: HostId,
    pub branch_id: DbId,
}

#[cfg(not(target_arch = "wasm32"))]
impl TryFrom<actix_web::dev::ConnectionInfo> for HostId {
    type Error = crate::errors::HostIdConversionError;

    fn try_from(value: actix_web::dev::ConnectionInfo) -> Result<Self, Self::Error> {
        // Prefer real ip even though it is not safe to use for security because we are
        // not using it for security just for pre-screening traffic to increase
        // the threshold required to do a DOS

        let addr = if let Some(realip_remote_addr) = value.realip_remote_addr() {
            realip_remote_addr
        } else if let Some(peer_addr) = value.peer_addr() {
            peer_addr
        } else {
            return Err(crate::errors::HostIdConversionError::NoPeerAddrFound);
        };
        Ok(addr.to_string().try_into()?)
    }
}
