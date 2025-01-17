/// If allowed empty not specified then empty is not allowed
#[macro_export]
macro_rules! string_wrapper {
    ($name: ident, $max_length: expr) => {
        #[derive(
            Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, Hash, PartialOrd, Ord,
        )]
        pub struct $name(String);

        impl TryFrom<String> for $name {
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
                let value = value.to_uppercase();
                Ok(Self(value))
            }
        }

        impl TryFrom<&str> for $name {
            type Error = ConversionError;

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
        impl sqlx::Encode<'_, Db> for $name {
            fn encode_by_ref(
                &self,
                buf: &mut <Db as sqlx::Database>::ArgumentBuffer<'_>,
            ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
                <String as sqlx::Encode<'_, Db>>::encode_by_ref(&self.0, buf)
            }
        }

        #[cfg(feature = "server_only")]
        impl sqlx::Type<Db> for $name {
            fn type_info() -> <Db as sqlx::Database>::TypeInfo {
                <String as sqlx::Type<Db>>::type_info()
            }
        }
    };
}
