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
    type Error = anyhow::Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value >= 0 {
            Ok(Self(value as u64))
        } else {
            anyhow::bail!("Negative values not supported as Id's. Value: {value}");
        }
    }
}

impl From<DbId> for u64 {
    fn from(value: DbId) -> Self {
        value.0
    }
}

#[cfg(feature = "server_only")]
pub mod sql {
    use super::*;
    use db_types::impl_encode_for_newtype_around_u64;

    impl_encode_for_newtype_around_u64!(DbId, "mysql", "postgres");
}
