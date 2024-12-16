use std::fmt::Display;

use anyhow::bail;
use chrono::NaiveDate;
use egui::WidgetText;

use crate::{errors::ConversionError, id::DbId};

use super::{Permissions, RoleIdAndName, RoleName};

#[derive(
    Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
/// Represents a username and is constrained to not be an empty string
pub struct Username(String);

#[derive(Default, Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct DisplayName(String);

impl TryFrom<String> for Username {
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

impl TryFrom<&str> for Username {
    type Error = ConversionError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.to_string().try_into()
    }
}

impl TryFrom<String> for DisplayName {
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

impl Username {
    pub const MAX_LENGTH: usize = 16;
}

impl DisplayName {
    pub const MAX_LENGTH: usize = 30;
}

impl From<Username> for String {
    fn from(value: Username) -> Self {
        value.0
    }
}

impl From<DisplayName> for String {
    fn from(value: DisplayName) -> Self {
        value.0
    }
}

impl AsRef<str> for Username {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for Username {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for DisplayName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
/// Stores the user info that is returned on login
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct UserInfo {
    pub username: Username,
    pub display_name: DisplayName,
    pub permissions: Permissions,
    pub branch_id: DbId,
}

/// Stores metadata about a user for representation on management screens
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct UserMetadata {
    pub username: Username,
    pub display_name: DisplayName,
    pub force_pass_change: bool,
    pub assigned_role: Option<DbId>,
    pub enabled: bool,
    pub locked_out: bool,
    pub failed_attempts: u8,
    pub pass_change_date: NaiveDate,
}
// TODO 6: We elected to not use user locking which means there is the
//          possibility of race conditions if multiple admins are editing the
//          same user but we decided that given that this is unlikely it's not
//          worth the extra effort and complexity that it would warrant to use
//          locking

/// Stores metadata diff for updating in the DB.
///
/// `Some` are the ones changed
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct UserMetadataDiff {
    /// Stores the user to be updated
    pub username: Username,
    pub display_name: Option<DisplayName>,
    pub force_pass_change: Option<bool>,
    pub assigned_role: Option<Option<DbId>>,
    pub enabled: Option<bool>,
    pub locked_out: Option<bool>,
    pub failed_attempts: Option<u8>,
}

/// A list of users and roles
///
/// Warning: Assumes that each assigned role ID has a corresponding role in the
/// list)
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct ListUsersRoles {
    pub users: Vec<UserMetadata>,
    pub roles: Vec<RoleIdAndName>,
}

impl ListUsersRoles {
    /// Converts a role ID into a Name using the contained list and errors if
    /// the ID cannot be found
    pub fn role_id_to_name(&self, id: DbId) -> anyhow::Result<&RoleName> {
        match self.roles.iter().find(|role| role.id == id) {
            Some(x) => Ok(&x.name),
            None => bail!("didn't find a role with ID: {id:?}"),
        }
    }
}

impl From<&Username> for WidgetText {
    fn from(value: &Username) -> Self {
        (&value.0).into()
    }
}

impl From<&DisplayName> for WidgetText {
    fn from(value: &DisplayName) -> Self {
        (&value.0).into()
    }
}

impl UserMetadata {
    pub fn same_username(&self, other: &Self) -> bool {
        self.username == other.username
    }
}

impl UserMetadataDiff {
    /// Returns an error if the usernames do not match
    ///
    /// Returns None if there are no differences otherwise
    /// sets the changed fields to `Some`
    pub fn from_diff(from: &UserMetadata, to: &UserMetadata) -> anyhow::Result<Option<Self>> {
        if !from.same_username(to) {
            bail!("user names do not match");
        }
        let username = from.username.clone();
        let display_name = if from.display_name == to.display_name {
            None
        } else {
            Some(to.display_name.clone())
        };
        let force_pass_change = if from.force_pass_change == to.force_pass_change {
            None
        } else {
            Some(to.force_pass_change)
        };
        let assigned_role = if from.assigned_role == to.assigned_role {
            None
        } else {
            Some(to.assigned_role)
        };
        let enabled = if from.enabled == to.enabled {
            None
        } else {
            Some(to.enabled)
        };
        let locked_out = if from.locked_out == to.locked_out {
            None
        } else {
            Some(to.locked_out)
        };
        let failed_attempts = if from.failed_attempts == to.failed_attempts {
            None
        } else {
            Some(to.failed_attempts)
        };
        Ok(
            if display_name.is_none()
                && force_pass_change.is_none()
                && assigned_role.is_none()
                && enabled.is_none()
                && locked_out.is_none()
                && failed_attempts.is_none()
            {
                None
            } else {
                Some(Self {
                    username,
                    display_name,
                    force_pass_change,
                    assigned_role,
                    enabled,
                    locked_out,
                    failed_attempts,
                })
            },
        )
    }

    pub fn is_valid(&self) -> anyhow::Result<()> {
        if self.display_name.is_some()
            || self.force_pass_change.is_some()
            || self.assigned_role.is_some()
            || self.enabled.is_some()
            || self.locked_out.is_some()
            || self.failed_attempts.is_some()
        {
            Ok(())
        } else {
            bail!("No change is being requested")
        }
    }
}

#[cfg(feature = "server_only")]
pub mod sql {
    use super::*;
    use crate::db_types::Db;

    impl sqlx::Encode<'_, Db> for Username {
        fn encode_by_ref(
            &self,
            buf: &mut <Db as sqlx::Database>::ArgumentBuffer<'_>,
        ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
            self.0.encode_by_ref(buf)
        }
    }

    impl sqlx::Encode<'_, Db> for DisplayName {
        fn encode_by_ref(
            &self,
            buf: &mut <Db as sqlx::Database>::ArgumentBuffer<'_>,
        ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
            self.0.encode_by_ref(buf)
        }
    }

    impl sqlx::Type<Db> for Username {
        fn type_info() -> <Db as sqlx::Database>::TypeInfo {
            String::type_info()
        }
    }

    impl sqlx::Type<Db> for DisplayName {
        fn type_info() -> <Db as sqlx::Database>::TypeInfo {
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
    #[case::too_long("a".repeat(17), ConversionError::MaxExceeded{max:16, actual:17})]
    fn illegal_username(#[case] name: String, #[case] expect: ConversionError) {
        // Act
        let actual: Result<Username, ConversionError> = name.try_into();

        // Assert
        assert_eq!(actual.unwrap_err(), expect);
    }

    #[rstest]
    #[case::empty("", ConversionError::Empty)]
    #[case::too_long("a".repeat(31), ConversionError::MaxExceeded{max:30, actual:31})]
    fn illegal_display_name(#[case] name: String, #[case] expect: ConversionError) {
        // Act
        let actual: Result<DisplayName, ConversionError> = name.try_into();

        // Assert
        assert_eq!(actual.unwrap_err(), expect);
    }
}
