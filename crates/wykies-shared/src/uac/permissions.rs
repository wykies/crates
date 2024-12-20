// TODO 5: Remove once depreciated permissions have been removed (may hide
// useful warnings but couldn't figure out how to make it more local)
#![allow(deprecated)]

use anyhow::bail;
use std::{
    collections::{BTreeSet, HashMap},
    fmt::{Debug, Display},
    sync::OnceLock,
};
use strum::{EnumCount, IntoEnumIterator};
use tracing::instrument;

use crate::const_config::path::*;

#[derive(
    Debug,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    strum::EnumCount,
    strum::EnumIter,
)]
pub enum Permission {
    // Record Transactions
    RecordManualTransaction,
    RecordDiscrepancy,

    // Transfers
    TransferRequest,
    TransferTo,
    TransferFrom,
    TransferView,
    TransferAny,
    TransferRemove,

    // Misc
    CustomsEntries,
    ViewShipmentManifest,
    #[deprecated]
    ChangePass,
    ImportData,
    ViewLog,
    ViewStockInfo,
    RunReports,
    Settings,
    NonCurrentDate,

    //Overrides
    GrantOverrideLocal,
    GrantOverrideRemote,

    // Management
    ManBranches,
    ManClasses,
    ManHostBranchAssignment,
    ManLines,
    ManMenu,
    ManMinMax,
    ManResetLocks,
    ManRoles,
    ManSpareParts,
    ManSuppliers,
    ManSupplierInvoices,
    ManUAC,

    // Import
    ImportStockLevelCountBatch,
    ImportTamsSync,
    ImportSales,
    ImportReceiving,
}

pub type PermissionMap = HashMap<&'static str, Vec<Permission>>;

static PERMISSION_MAP: OnceLock<PermissionMap> = OnceLock::new();

pub fn default_permissions() -> PermissionMap {
    use Permission as perm;
    let mut result: HashMap<&str, Vec<Permission>> = HashMap::new();
    result.insert(PATH_API_ADMIN_BRANCH_CREATE.path, vec![perm::ManBranches]);
    result.insert(
        PATH_API_ADMIN_HOSTBRANCH_LIST.path,
        vec![perm::ManHostBranchAssignment],
    );
    result.insert(
        PATH_API_ADMIN_HOSTBRANCH_SET.path,
        vec![perm::ManHostBranchAssignment],
    );
    result.insert(PATH_API_ADMIN_ROLE_ASSIGN.path, vec![perm::ManUAC]);
    result.insert(PATH_API_ADMIN_ROLE_CREATE.path, vec![perm::ManRoles]);
    result.insert(PATH_API_ADMIN_ROLE.path, vec![perm::ManRoles]);
    result.insert(PATH_API_ADMIN_USER_NEW.path, vec![perm::ManUAC]);
    result.insert(PATH_API_ADMIN_USER_PASSWORD_RESET.path, vec![perm::ManUAC]);
    result.insert(PATH_API_ADMIN_USER_UPDATE.path, vec![perm::ManUAC]);
    result.insert(PATH_API_ADMIN_USER.path, vec![perm::ManUAC]);
    result.insert(PATH_API_ADMIN_USERS_LIST_AND_ROLES.path, vec![perm::ManUAC]);
    result.insert(PATH_API_CHANGE_PASSWORD.path, vec![]);
    result.insert(PATH_API_HOSTBRANCH_LOOKUP.path, vec![]);
    result.insert(PATH_API_LOGOUT.path, vec![]);
    result.insert(PATH_WS_TOKEN_CHAT.path, vec![]); // Included here because it's shared by all current applications and will give
                                                    // a 404 if the path is not registered so shouldn't hurt
    result
}

/// Only sets the permissions if they haven't already been set
pub fn try_set_permissions(value: PermissionMap) -> Result<(), PermissionMap> {
    PERMISSION_MAP.set(value)
}

/// Initializes the permissions may be run more than once without issue (will
/// only have an effect the first time)
pub fn init_permissions_to_defaults() {
    // Set permissions and ignore if they were already set
    let _ = try_set_permissions(default_permissions());
}

/// Takes a path and returns the permissions required for it if found
///
/// **Note:** All paths that require login must have permissions set to be
/// accessed even if it is 0 permissions
#[tracing::instrument(ret)]
pub fn get_required_permissions(path: &str) -> Option<&'static [Permission]> {
    PERMISSION_MAP
        .get()
        .expect("permissions were not initialized")
        .get(path)
        .map(|x| &x[..])
}

impl Permissions {
    pub fn includes(&self, perms: &[Permission]) -> bool {
        perms.iter().all(|x| self.0.contains(x))
    }

    #[instrument(ret, err(Debug))]
    pub fn is_allowed_access(&self, path: &str) -> anyhow::Result<bool> {
        match get_required_permissions(path) {
            Some(required_permissions) => Ok(self.includes(required_permissions)),
            None => bail!("lookup of permissions for other endpoint failed"),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Permissions(pub BTreeSet<Permission>);

impl From<Vec<Permission>> for Permissions {
    fn from(value: Vec<Permission>) -> Self {
        let mut result: Self = Default::default();
        for permission in value.into_iter() {
            result.0.insert(permission);
        }
        result
    }
}

impl TryFrom<String> for Permissions {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Ok(Self::default());
        }

        if Permission::COUNT != value.len() {
            bail!("Only valid strings are those of length {} but found string of length {}. Value: {value:?}", Permission::COUNT , value.len());
        }
        let mut result = Permissions::default();
        for (c, p) in value.chars().zip(Permission::iter()) {
            match c {
                '0' => (), // Do nothing this one is not included
                '1' => {
                    let did_not_exist = result.0.insert(p);
                    debug_assert!(did_not_exist, "Since we should always get a new Permission we should never already have the value inserted");
                }
                _ => bail!(
                    "found an unexpected character for {p:?}. Only 0 or 1 expected but found {c}"
                ),
            }
        }
        Ok(result)
    }
}

impl From<&Permissions> for String {
    fn from(value: &Permissions) -> Self {
        let mut iter = value.0.iter();
        let mut next = iter.next();
        let mut result = String::with_capacity(Permission::COUNT);
        for permission in Permission::iter() {
            let ch = match next.as_ref() {
                Some(&x) if x == &permission => {
                    next = iter.next();
                    debug_assert!(
                        next.is_none() || next.as_ref().is_some_and(|&x| x > &permission),
                        "Implementation assumes sorted values from iterator but assumption violated. Next: {next:?} Current: {permission}"
                    );
                    '1'
                }
                _ => '0',
            };
            result.push(ch);
        }
        debug_assert_eq!(result.len(), Permission::COUNT);
        debug_assert!(next.is_none());
        result
    }
}

impl From<Permissions> for String {
    fn from(value: Permissions) -> Self {
        (&value).into()
    }
}

impl Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display_text = match self {
            Permission::RecordManualTransaction => "Record Manual Transaction",
            Permission::RecordDiscrepancy => "Record Discrepancy",
            Permission::TransferRequest => "Transfers - Create Request",
            Permission::TransferTo => "Transfers - Prepare Order",
            Permission::TransferFrom => "Transfers - Receive Order",
            Permission::TransferView => "Transfers - View",
            Permission::TransferAny => "Transfers Set Any Location",
            Permission::TransferRemove => "Delete Transfers",
            Permission::CustomsEntries => "CustomsEntries",
            Permission::ViewShipmentManifest => "View Shipment Manifest",
            Permission::ChangePass => "Change Password",
            Permission::ImportData => "Import Data",
            Permission::ViewLog => "View Log",
            Permission::ViewStockInfo => "View Stock Info",
            Permission::RunReports => "Run Reports",
            Permission::Settings => "Settings",
            Permission::NonCurrentDate => "Record Transaction on Non-Current Date",
            Permission::GrantOverrideLocal => "Grant Override Local",
            Permission::GrantOverrideRemote => "Grant Override Remote",
            Permission::ManBranches => "Manage Branches",
            Permission::ManClasses => "Manage Classes",
            Permission::ManHostBranchAssignment => "Manage Host Branch Assignment",
            Permission::ManLines => "Manage Lines",
            Permission::ManMenu => "Management Menu",
            Permission::ManMinMax => "Manage Min/Max",
            Permission::ManResetLocks => "Reset Locks",
            Permission::ManRoles => "Manage Roles",
            Permission::ManSpareParts => "Manage Spare Parts",
            Permission::ManSuppliers => "Manage Suppliers",
            Permission::ManSupplierInvoices => "Manage Supplier Invoices",
            Permission::ManUAC => "Manage User Account Control",
            Permission::ImportStockLevelCountBatch => "Import - Stock Level Count Batch",
            Permission::ImportTamsSync => "Import - Tams Sync",
            Permission::ImportSales => "Import - Sales",
            Permission::ImportReceiving => "Import - Receiving",
        };
        write!(f, "{display_text}")
    }
}

impl Debug for Permissions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text: String = self.into();
        f.debug_tuple("Permissions").field(&text).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use Permission as p;

    #[rstest]
    #[case::empty("00000000000000000000000000000000000", vec![])]
    #[case::administrator("11111101111111111111111111111111111", vec![p::RecordManualTransaction, p::RecordDiscrepancy, p::TransferRequest, p::TransferTo, p::TransferFrom, p::TransferView, p::TransferRemove, p::CustomsEntries, p::ViewShipmentManifest, p::ChangePass, p::ImportData, p::ViewLog, p::ViewStockInfo, p::RunReports, p::Settings, p::NonCurrentDate, p::GrantOverrideLocal, p::GrantOverrideRemote, p::ManBranches, p::ManClasses, p::ManHostBranchAssignment, p::ManLines, p::ManMenu, p::ManMinMax, p::ManResetLocks, p::ManRoles, p::ManSpareParts, p::ManSuppliers, p::ManSupplierInvoices, p::ManUAC, p::ImportStockLevelCountBatch, p::ImportTamsSync, p::ImportSales, p::ImportReceiving])]
    #[case::view_only("00000100001001000000000000000000000", vec![p::TransferView, p::ChangePass, p::ViewStockInfo])]
    #[case::request_transfer("00100100001001000000000000000000000", vec![p::TransferRequest, p::TransferView, p::ChangePass, p::ViewStockInfo])]
    #[case::prepare_transfer("00010100001001000000000000000000000", vec![p::TransferTo, p::TransferView, p::ChangePass, p::ViewStockInfo])]
    #[case::receive_transfer("00001100001001000000000000000000000", vec![p::TransferFrom, p::TransferView, p::ChangePass, p::ViewStockInfo])]
    #[case::transfer_admin("00111101001001100000000000000000000", vec![p::TransferRequest, p::TransferTo, p::TransferFrom, p::TransferView, p::ChangePass, p::TransferRemove, p::ViewStockInfo, p::RunReports])]
    #[case::transfers_all("01111100001101100000000000000001000", vec![p::RecordDiscrepancy, p::TransferRequest, p::TransferTo, p::TransferFrom, p::TransferView, p::ChangePass, p::ImportData, p::ViewStockInfo, p::RunReports, p::ImportStockLevelCountBatch])]
    fn string_to_permissions(#[case] s: String, #[case] permission_list: Vec<Permission>) {
        // Arrange
        let expected: Permissions = permission_list.into();

        // Act
        let actual: Permissions = s.clone().try_into().unwrap();

        // Assert
        assert_eq!(actual, expected);

        // Arrange - Test reverse
        let expected = s;
        let input = actual;

        // Act
        let actual: String = input.into();

        // Assert
        assert_eq!(actual, expected);
    }

    #[rstest]
    #[case("10000000000000000000000000000000000", p::RecordManualTransaction)]
    #[case("01000000000000000000000000000000000", p::RecordDiscrepancy)]
    #[case("00100000000000000000000000000000000", p::TransferRequest)]
    #[case("00010000000000000000000000000000000", p::TransferTo)]
    #[case("00001000000000000000000000000000000", p::TransferFrom)]
    #[case("00000100000000000000000000000000000", p::TransferView)]
    #[case("00000010000000000000000000000000000", p::TransferAny)]
    #[case("00000001000000000000000000000000000", p::TransferRemove)]
    #[case("00000000100000000000000000000000000", p::CustomsEntries)]
    #[case("00000000010000000000000000000000000", p::ViewShipmentManifest)]
    #[case("00000000001000000000000000000000000", p::ChangePass)]
    #[case("00000000000100000000000000000000000", p::ImportData)]
    #[case("00000000000010000000000000000000000", p::ViewLog)]
    #[case("00000000000001000000000000000000000", p::ViewStockInfo)]
    #[case("00000000000000100000000000000000000", p::RunReports)]
    #[case("00000000000000010000000000000000000", p::Settings)]
    #[case("00000000000000001000000000000000000", p::NonCurrentDate)]
    #[case("00000000000000000100000000000000000", p::GrantOverrideLocal)]
    #[case("00000000000000000010000000000000000", p::GrantOverrideRemote)]
    #[case("00000000000000000001000000000000000", p::ManBranches)]
    #[case("00000000000000000000100000000000000", p::ManClasses)]
    #[case("00000000000000000000010000000000000", p::ManHostBranchAssignment)]
    #[case("00000000000000000000001000000000000", p::ManLines)]
    #[case("00000000000000000000000100000000000", p::ManMenu)]
    #[case("00000000000000000000000010000000000", p::ManMinMax)]
    #[case("00000000000000000000000001000000000", p::ManResetLocks)]
    #[case("00000000000000000000000000100000000", p::ManRoles)]
    #[case("00000000000000000000000000010000000", p::ManSpareParts)]
    #[case("00000000000000000000000000001000000", p::ManSuppliers)]
    #[case("00000000000000000000000000000100000", p::ManSupplierInvoices)]
    #[case("00000000000000000000000000000010000", p::ManUAC)]
    #[case("00000000000000000000000000000001000", p::ImportStockLevelCountBatch)]
    #[case("00000000000000000000000000000000100", p::ImportTamsSync)]
    #[case("00000000000000000000000000000000010", p::ImportSales)]
    #[case("00000000000000000000000000000000001", p::ImportReceiving)]
    fn string_to_permission(#[case] s: String, #[case] permission: Permission) {
        println!("Inputs are String: {s} and permission: {permission:?} ({permission})"); // This print is included to make it easier to identify which test
        let mut expected: Permissions = Permissions::default();
        expected.0.insert(permission);
        let actual: Permissions = s.try_into().unwrap();
        assert_eq!(
            actual,
            expected,
            "Permission as Text: '{}'",
            expected.0.first().unwrap()
        );
    }

    #[rstest]
    #[case::too_short("111")]
    #[case::invalid_char("a0000100001001000000000000000000000")]
    fn invalid_inputs(#[case] s: String) {
        let actual: anyhow::Result<Permissions> = s.try_into();
        match actual {
            Ok(val) => panic!("Expected an error but got {val:?}"),
            Err(e) => println!("Expected and error and got one: {e}"),
        }
    }

    /// Just a sanity check that any admin paths in default require "management
    /// permissions"
    #[test]
    fn admin_paths_require_management_permission() {
        PERMISSION_MAP
            .set(default_permissions())
            .expect("failed to set to default permissions");
        assert!(
            PERMISSION_MAP
                .get()
                .unwrap()
                .iter()
                .any(|(path, _)| { path.contains("admin") }),
            "At least one permission must be a admin permission"
        );

        // All permissions are either not admin or they have 'Manage' in at least one of
        // their required permissions
        for (path, permissions) in PERMISSION_MAP.get().unwrap().iter() {
            if path.contains("admin")
                && !permissions
                    .iter()
                    .any(|perm| dbg!(perm.to_string()).contains("Manage"))
            {
                panic!("Failed to find `Manage` in any of the permissions for path.\npath: {path:?}\npermissions: {permissions:?}")
            }
        }
    }
}
