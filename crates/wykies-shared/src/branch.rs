use crate::{errors::ConversionError, id::DbId};

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct Branch {
    pub id: DbId,
    pub name: BranchName,
    pub address: BranchAddress,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct BranchDraft {
    pub name: BranchName,
    pub address: BranchAddress,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct BranchName(String);

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct BranchAddress(String);

impl BranchName {
    const MAX_LENGTH: usize = 30;
}

impl TryFrom<String> for BranchName {
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

impl From<BranchName> for String {
    fn from(value: BranchName) -> Self {
        value.0
    }
}

impl BranchAddress {
    const MAX_LENGTH: usize = 200;
}

impl TryFrom<String> for BranchAddress {
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

impl From<BranchAddress> for String {
    fn from(value: BranchAddress) -> Self {
        value.0
    }
}

#[cfg(feature = "server_only")]
pub mod sql {
    use super::*;
    use crate::db_types::Db;

    impl sqlx::Encode<'_, Db> for BranchName {
        fn encode_by_ref(
            &self,
            buf: &mut <Db as sqlx::Database>::ArgumentBuffer<'_>,
        ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
            <String as sqlx::Encode<'_, Db>>::encode_by_ref(&self.0, buf)
        }
    }

    impl sqlx::Encode<'_, Db> for BranchAddress {
        fn encode_by_ref(
            &self,
            buf: &mut <Db as sqlx::Database>::ArgumentBuffer<'_>,
        ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
            <String as sqlx::Encode<'_, Db>>::encode_by_ref(&self.0, buf)
        }
    }

    impl sqlx::Type<Db> for BranchName {
        fn type_info() -> <Db as sqlx::Database>::TypeInfo {
            <String as sqlx::Type<Db>>::type_info()
        }
    }

    impl sqlx::Type<Db> for BranchAddress {
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
    #[case::too_long("b".repeat(31), ConversionError::MaxExceeded{max:30, actual:31})]
    fn illegal_branch_name(#[case] name: String, #[case] expect: ConversionError) {
        // Act
        let actual: Result<BranchName, ConversionError> = name.try_into();

        // Assert
        assert_eq!(actual.unwrap_err(), expect);
    }

    #[rstest]
    #[case::too_long("b".repeat(201), ConversionError::MaxExceeded{max:200, actual:201})]
    fn illegal_branch_address(#[case] name: String, #[case] expect: ConversionError) {
        // Act
        let actual: Result<BranchAddress, ConversionError> = name.try_into();

        // Assert
        assert_eq!(actual.unwrap_err(), expect);
    }
}
