use crate::errors::DbIdConversionError;

#[derive(
    Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Copy,
)]
pub struct DbId(u64);

impl From<u64> for DbId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl TryFrom<i32> for DbId {
    type Error = DbIdConversionError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value >= 0 {
            Ok(Self(value as u64))
        } else {
            Err(DbIdConversionError::NegativeI32(value))
        }
    }
}

impl From<DbId> for u64 {
    fn from(value: DbId) -> Self {
        value.0
    }
}

impl TryFrom<DbId> for i32 {
    type Error = DbIdConversionError;

    fn try_from(value: DbId) -> Result<Self, Self::Error> {
        value
            .0
            .try_into()
            .map_err(|_| DbIdConversionError::TooBigForI32(value))
    }
}

#[cfg(feature = "server_only")]
pub mod sql {
    use super::*;
    use db_types::impl_encode_for_newtype_around_u64;

    impl_encode_for_newtype_around_u64!(DbId, "mysql", "postgres");
}
