use std::{ops::Deref, sync::LazyLock};

use egui::WidgetText;
use serde::{Deserialize, Serialize};

use crate::{errors::ConversionError, id::DbId};

use super::Permissions;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Role {
    pub id: DbId,
    pub name: RoleName,
    pub description: RoleDescription,
    pub permissions: Permissions,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RoleIdAndName {
    pub id: DbId,
    pub name: RoleName,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RoleDraft {
    pub name: RoleName,
    pub description: RoleDescription,
    pub permissions: Permissions,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RoleName(String);

impl RoleName {
    pub const MAX_LENGTH: usize = 16;

    pub fn no_role_set() -> &'static Self {
        static RESULT: LazyLock<RoleName> = LazyLock::new(|| {
            RoleName::try_from("[NOT SET]".to_string()).expect("test below ensure this is valid")
        });
        &RESULT
    }
}

impl From<&RoleName> for WidgetText {
    fn from(value: &RoleName) -> Self {
        (&value.0).into()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RoleDescription(String);

impl RoleDescription {
    pub const MAX_LENGTH: usize = 50;
}

impl TryFrom<String> for RoleName {
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

impl TryFrom<String> for RoleDescription {
    type Error = ConversionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() > Self::MAX_LENGTH {
            return Err(ConversionError::MaxExceeded {
                max: Self::MAX_LENGTH,
                actual: value.len(),
            });
        }
        Ok(Self(value))
    }
}

impl From<RoleName> for String {
    fn from(value: RoleName) -> Self {
        value.0
    }
}

impl From<RoleDescription> for String {
    fn from(value: RoleDescription) -> Self {
        value.0
    }
}

impl Deref for RoleName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0[..]
    }
}

impl Deref for RoleDescription {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0[..]
    }
}

#[cfg(feature = "server_only")]
pub mod sql {
    use super::*;
    use crate::db_types::Db;

    impl sqlx::Encode<'_, Db> for RoleName {
        fn encode_by_ref(
            &self,
            buf: &mut <Db as sqlx::Database>::ArgumentBuffer<'_>,
        ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
            <String as sqlx::Encode<'_, Db>>::encode_by_ref(&self.0, buf)
        }
    }

    impl sqlx::Encode<'_, Db> for RoleDescription {
        fn encode_by_ref(
            &self,
            buf: &mut <Db as sqlx::Database>::ArgumentBuffer<'_>,
        ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
            <String as sqlx::Encode<'_, Db>>::encode_by_ref(&self.0, buf)
        }
    }

    impl sqlx::Type<Db> for RoleName {
        fn type_info() -> <Db as sqlx::Database>::TypeInfo {
            <String as sqlx::Type<Db>>::type_info()
        }
    }

    impl sqlx::Type<Db> for RoleDescription {
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
    #[case::too_long("123456789+1234567", ConversionError::MaxExceeded{max:16, actual:17})]
    fn illegal_role_names(#[case] name: String, #[case] expect: ConversionError) {
        // Act
        let actual: Result<RoleName, ConversionError> = name.try_into();

        // Assert
        assert_eq!(actual.unwrap_err(), expect);
    }

    #[test]
    fn illegal_role_description() {
        // Act
        let actual: Result<RoleDescription, ConversionError> = "a".repeat(51).try_into();

        // Assert
        assert_eq!(
            actual.unwrap_err(),
            ConversionError::MaxExceeded {
                max: 50,
                actual: 51
            }
        );
    }

    #[test]
    fn no_role_set_is_valid_value() {
        println!("{:?}", RoleName::no_role_set())
    }
}
