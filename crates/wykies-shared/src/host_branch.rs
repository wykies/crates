use std::fmt::Display;

use crate::{errors::ConversionError, id::DbId};

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct HostBranchPair {
    pub host_id: HostId,
    pub branch_id: DbId,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct HostId(String);

impl HostId {
    const MAX_LENGTH: usize = 50;
}

impl AsRef<str> for HostId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for HostId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for HostId {
    type Error = ConversionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(ConversionError::Empty);
        }
        if value.len() > Self::MAX_LENGTH {
            return Err(ConversionError::MaxExceeded {
                max: Self::MAX_LENGTH,
                actual: value.len(),
            });
        }
        Ok(Self(value))
    }
}

impl From<HostId> for String {
    fn from(value: HostId) -> Self {
        value.0
    }
}

#[cfg(not(target_arch = "wasm32"))]
use anyhow::{bail, Context as _};

#[cfg(not(target_arch = "wasm32"))]
impl TryFrom<actix_web::dev::ConnectionInfo> for HostId {
    type Error = anyhow::Error;

    fn try_from(value: actix_web::dev::ConnectionInfo) -> Result<Self, Self::Error> {
        // Prefer real ip even though it is not safe to use for security because we are not using it for security
        // just for pre-screening traffic to increase the threshold required to do a DOS

        let addr = if let Some(realip_remote_addr) = value.realip_remote_addr() {
            realip_remote_addr
        } else if let Some(peer_addr) = value.peer_addr() {
            peer_addr
        } else {
            bail!("No 'peer_addr' found");
        };
        addr.to_string().try_into().context("invalid host_id")
    }
}

#[cfg(feature = "server_only")]
pub mod sql {
    use super::*;
    use crate::db_types::Db;

    impl sqlx::Encode<'_, Db> for HostId {
        fn encode_by_ref(
            &self,
            buf: &mut <Db as sqlx::Database>::ArgumentBuffer<'_>,
        ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
            <String as sqlx::Encode<'_, Db>>::encode_by_ref(&self.0, buf)
        }
    }

    impl sqlx::Type<Db> for HostId {
        fn type_info() -> <Db as sqlx::Database>::TypeInfo {
            <String as sqlx::Type<Db>>::type_info()
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::empty("", ConversionError::Empty)]
    #[case::too_long("b".repeat(51), ConversionError::MaxExceeded{max:50, actual:51})]
    fn illegal_role_names(#[case] name: String, #[case] expect: ConversionError) {
        // Act
        let actual: Result<HostId, ConversionError> = name.try_into();

        // Assert
        assert_eq!(actual.unwrap_err(), expect);
    }
}
