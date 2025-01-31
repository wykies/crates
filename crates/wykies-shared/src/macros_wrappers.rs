#[macro_export]
macro_rules! string_wrapper {
    ($name: ident, $max_length: expr, $always_case: expr) => {
        #[derive(
            Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, Hash, PartialOrd, Ord,
        )]
        pub struct $name(String);

        impl TryFrom<String> for $name {
            type Error = ConversionError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.is_empty() {
                    return Err(ConversionError::Empty{type_name: stringify!($name)});
                }
                if value.len() > Self::MAX_LENGTH {
                    return Err(ConversionError::MaxExceeded {
                        max: Self::MAX_LENGTH,
                        actual: value.len(),
                        type_name: stringify!($name),
                    });
                }
                let value = match $always_case {
                    AlwaysCase::Any => value,
                    AlwaysCase::Lower => value.to_lowercase(),
                    AlwaysCase::Upper => value.to_uppercase(),
                };

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

            pub fn try_from_opt(value: Option<String>) -> Result<Option<Self>, ConversionError> {
                match value {
                    Some(value) if value.is_empty() => Ok(None),
                    Some(value) => Ok(Some(value.try_into()?)),
                    None => Ok(None),
                }
            }
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

#[macro_export]
macro_rules! char_array_wrapper {
    ($name: ident, $length: expr, $always_case: expr) => {
        #[derive(
            Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord,
        )]
        pub struct $name([char; Self::LENGTH]);

        impl $name {
            pub const LENGTH: usize = $length;

            pub fn try_from_opt(value: Option<String>) -> Result<Option<Self>, ConversionError> {
                match value {
                    Some(value) if value.is_empty() => Ok(None),
                    Some(value) => Ok(Some(value.try_into()?)),
                    None => Ok(None),
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut result = String::new();
                for ch in self.0.iter().cloned().filter(|&c| c != ' ') {
                    result.push(ch);
                }

                write!(f, "{result}")
            }
        }

        impl TryFrom<String> for $name {
            type Error = ConversionError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.is_empty() {
                    return Err(ConversionError::Empty {
                        type_name: stringify!($name),
                    });
                }
                if value.len() > Self::LENGTH {
                    return Err(ConversionError::MaxExceeded {
                        max: Self::LENGTH,
                        actual: value.len(),
                        type_name: stringify!($name),
                    });
                }
                let value = match $always_case {
                    AlwaysCase::Any => value,
                    AlwaysCase::Lower => value.to_lowercase(),
                    AlwaysCase::Upper => value.to_uppercase(),
                };

                let mut char_array = [' '; Self::LENGTH];
                for (i, c) in value.char_indices() {
                    assert!(
                        i <= Self::LENGTH,
                        "input was not properly validated, max length exceeded"
                    );
                    char_array[i] = c;
                }
                Ok(Self(char_array))
            }
        }

        impl TryFrom<&str> for $name {
            type Error = ConversionError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                value.to_string().try_into()
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.to_string()
            }
        }

        impl From<&$name> for egui::WidgetText {
            fn from(value: &$name) -> Self {
                value.to_string().into()
            }
        }

        #[cfg(feature = "server_only")]
        impl sqlx::Encode<'_, Db> for $name {
            fn encode_by_ref(
                &self,
                buf: &mut <Db as sqlx::Database>::ArgumentBuffer<'_>,
            ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
                <String as sqlx::Encode<'_, Db>>::encode_by_ref(&self.to_string(), buf)
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

#[macro_export]
macro_rules! id_wrapper {
    ($name: ident, $error_name: ident) => {
        #[derive(Debug, thiserror::Error, PartialEq, Eq)]
        pub enum $error_name {
            #[error("Negative values not supported as Id's. Value: {0}")]
            NegativeI32(i32),
            #[error("Internal value of $name is too large for i32. Value: {0:?}")]
            TooBigForI32($name),
            #[error("Unable to convert str into $name: {0:?}")]
            InvalidStr(#[from] std::num::ParseIntError),
        }

        #[derive(
            Debug,
            serde::Serialize,
            serde::Deserialize,
            Clone,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Copy,
            Hash,
        )]
        pub struct $name(u64);

        impl From<u64> for $name {
            fn from(value: u64) -> Self {
                Self(value)
            }
        }

        impl TryFrom<i32> for $name {
            type Error = $error_name;

            fn try_from(value: i32) -> Result<Self, Self::Error> {
                if value >= 0 {
                    Ok(Self(value as u64))
                } else {
                    Err($error_name::NegativeI32(value))
                }
            }
        }

        impl From<$name> for u64 {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl TryFrom<&str> for $name {
            type Error = $error_name;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                let value: u64 = value.parse()?;
                Ok(Self(value))
            }
        }

        impl TryFrom<$name> for i32 {
            type Error = $error_name;

            fn try_from(value: $name) -> Result<Self, Self::Error> {
                value
                    .0
                    .try_into()
                    .map_err(|_| $error_name::TooBigForI32(value))
            }
        }

        #[cfg(feature = "server_only")]
        db_types::impl_encode_for_newtype_around_u64!($name, "mysql", "postgres");

        #[cfg(feature = "server_only")]
        impl actix_web::error::ResponseError for $error_name {
            fn status_code(&self) -> actix_web::http::StatusCode {
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    };
}

pub enum AlwaysCase {
    Any,
    Lower,
    Upper,
}
