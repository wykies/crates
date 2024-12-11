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
        Ok(match value.peer_addr() {
            Some(x) => x.to_string().try_into().context("invalid host_id")?,
            None => bail!("No 'peer_addr' found"),
        })
    }
}

#[cfg(feature = "server_only")]
pub mod sql {
    use super::*;

    impl sqlx::Encode<'_, sqlx::MySql> for HostId {
        fn encode_by_ref(
            &self,
            buf: &mut <sqlx::MySql as sqlx::Database>::ArgumentBuffer<'_>,
        ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
            self.0.encode_by_ref(buf)
        }
    }

    impl sqlx::Type<sqlx::MySql> for HostId {
        fn type_info() -> <sqlx::MySql as sqlx::Database>::TypeInfo {
            String::type_info()
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
