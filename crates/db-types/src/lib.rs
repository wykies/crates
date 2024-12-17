#[cfg(not(feature = "mysql"))]
compile_error!("No Database Type selected. Please use create features to select a DB to use");

#[cfg_attr(feature = "mysql", path = "db_types_mysql.rs")]
#[cfg_attr(
    all(not(feature = "mysql"), feature = "postgres"),
    path = "db_types_postgres.rs"
)]
mod types;

pub use types::*;

#[macro_export]
macro_rules! impl_encode_for_newtype_around_u64 {
    ($type: ty, $mysql_feature: expr, $postgres_feature: expr) => {
        #[cfg(feature = $mysql_feature)]
        impl sqlx::Encode<'_, sqlx::MySql> for $type {
            fn encode_by_ref(
                &self,
                buf: &mut <sqlx::MySql as sqlx::Database>::ArgumentBuffer<'_>,
            ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
                self.0.encode_by_ref(buf)
            }
        }

        #[cfg(feature = $postgres_feature)]
        impl sqlx::Encode<'_, sqlx::Postgres> for $type {
            fn encode_by_ref(
                &self,
                buf: &mut <sqlx::Postgres as sqlx::Database>::ArgumentBuffer<'_>,
            ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
                let converted: i64 = self
                    .0
                    .try_into()
                    .expect("failed to convert from u64 to i64");
                <i64 as sqlx::Encode<'_, sqlx::Postgres>>::encode_by_ref(&converted, buf)
            }
        }

        #[cfg(feature = $mysql_feature)]
        impl sqlx::Type<sqlx::MySql> for $type {
            fn type_info() -> <sqlx::MySql as sqlx::Database>::TypeInfo {
                u64::type_info()
            }
        }

        #[cfg(feature = $postgres_feature)]
        impl sqlx::Type<sqlx::Postgres> for $type {
            fn type_info() -> <sqlx::Postgres as sqlx::Database>::TypeInfo {
                <i64 as sqlx::Type<sqlx::Postgres>>::type_info()
            }
        }
    };
}
