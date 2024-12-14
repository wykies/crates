use wykies_shared::uac::{Permission, PermissionsError};

use crate::helpers::{no_cb, spawn_app};

#[tokio::test]
async fn unprivileged_user_is_denied() {
    // Arrange
    let app = spawn_app().await;

    // Act - Login
    app.login_assert().await;

    // Act - Attempt to access restricted endpoint
    let actual = app
        .core_client
        .get_list_host_branch_pairs(no_cb)
        .await
        .unwrap();

    // Assert - Ensure request was denied
    let expected_error =
        PermissionsError::MissingPermissions(vec![Permission::ManHostBranchAssignment]);
    assert_eq!(actual.unwrap_err().to_string(), expected_error.to_string());
}

#[tokio::test]
async fn test_admin_user_works() {
    // Arrange
    let app_admin = spawn_app().await.create_admin_user().await;

    // Act - Login
    app_admin.login_assert().await;

    // Act - Attempt to access restricted endpoint
    let actual = app_admin
        .core_client
        .get_list_host_branch_pairs(no_cb)
        .await
        .unwrap();

    // Assert - Ensure request succeeded
    actual.unwrap();
}
