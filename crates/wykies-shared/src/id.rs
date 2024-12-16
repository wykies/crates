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
    use crate::db_types::Db;

    impl sqlx::Encode<'_, Db> for DbId {
        fn encode_by_ref(
            &self,
            buf: &mut <Db as sqlx::Database>::ArgumentBuffer<'_>,
        ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
            self.0.encode_by_ref(buf)
        }
    }

    impl sqlx::Type<Db> for DbId {
        fn type_info() -> <Db as sqlx::Database>::TypeInfo {
            u64::type_info()
        }
    }
}
