use super::Permissions;
#[cfg(feature = "server_only")]
use crate::db_types::Db;
use crate::{errors::ConversionError, id::DbId, string_wrapper};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

string_wrapper!(RoleName, 16);
string_wrapper!(RoleDescription, 50);

impl RoleName {
    pub fn no_role_set() -> &'static Self {
        static RESULT: LazyLock<RoleName> = LazyLock::new(|| {
            RoleName::try_from("[NOT SET]".to_string()).expect("test below ensure this is valid")
        });
        &RESULT
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_role_set_is_valid_value() {
        println!("{:?}", RoleName::no_role_set())
    }
}
