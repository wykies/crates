/// If allowed empty not specified then empty is not allowed
#[macro_export]
macro_rules! string_wrapper {
    ($name: ident, $max_length: expr) => {
        #[derive(
            Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, Hash, PartialOrd, Ord,
        )]
        pub struct $name(String);

        impl TryFrom<String> for $name {
            type Error = crate::errors::ConversionError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.is_empty() {
                    return Err(crate::errors::ConversionError::Empty);
                }
                if value.len() > Self::MAX_LENGTH {
                    return Err(crate::errors::ConversionError::MaxExceeded {
                        max: Self::MAX_LENGTH,
                        actual: value.len(),
                    });
                }
                Ok(Self(value))
            }
        }

        impl TryFrom<&str> for $name {
            type Error = crate::errors::ConversionError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                value.to_string().try_into()
            }
        }

        impl $name {
            pub const MAX_LENGTH: usize = $max_length;
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<&$name> for egui::WidgetText {
            fn from(value: &$name) -> Self {
                (&value.0).into()
            }
        }

        impl std::ops::Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                &self.0[..]
            }
        }

        #[cfg(feature = "server_only")]
        impl sqlx::Encode<'_, crate::db_types::Db> for $name {
            fn encode_by_ref(
                &self,
                buf: &mut <crate::db_types::Db as sqlx::Database>::ArgumentBuffer<'_>,
            ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
                <String as sqlx::Encode<'_, crate::db_types::Db>>::encode_by_ref(&self.0, buf)
            }
        }

        #[cfg(feature = "server_only")]
        impl sqlx::Type<crate::db_types::Db> for $name {
            fn type_info() -> <crate::db_types::Db as sqlx::Database>::TypeInfo {
                <String as sqlx::Type<crate::db_types::Db>>::type_info()
            }
        }
    };
}
