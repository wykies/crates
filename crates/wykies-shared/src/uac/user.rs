use super::{Permissions, RoleIdAndName, RoleName};
#[cfg(feature = "server_only")]
use crate::db_types::Db;
use crate::{errors::ConversionError, id::DbId, string_wrapper, AlwaysCase};
use anyhow::bail;
use chrono::NaiveDate;

string_wrapper!(Username, 16, AlwaysCase::Any);
string_wrapper!(DisplayName, 30, AlwaysCase::Any);

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

#[cfg(test)]
mod tests {
    // Actually test the macro so we only need one of these
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
}
