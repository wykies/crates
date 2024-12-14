use crate::helpers::{no_cb, spawn_app};
use wykies_shared::{
    req_args::api::admin::role::AssignReqArgs,
    uac::{Permission, Role, RoleDraft},
};

#[tokio::test]
async fn create_and_assign_role_to_user() {
    // Arrange
    let app_normal = spawn_app().await;
    let app_admin = app_normal.create_admin_user().await;
    let role_draft = RoleDraft {
        name: "Test Role".to_string().try_into().unwrap(),
        description: "Test Description".to_string().try_into().unwrap(),
        permissions: vec![
            Permission::ManHostBranchAssignment,
            Permission::RecordDiscrepancy,
        ]
        .into(),
    };

    // Act - Login the admin
    app_admin.login_assert().await;

    // Act - Create Role
    let role_id = app_admin
        .core_client
        .create_role(&role_draft, no_cb)
        .await
        .expect("failed to receive on rx")
        .expect("failed to extract role_id");

    // Assert - Verify Role was created
    let role = app_admin
        .core_client
        .get_role(role_id, no_cb)
        .await
        .expect("failed to receive on rx")
        .expect("failed to extract role_id");
    let expected = Role {
        id: role_id,
        name: role_draft.name,
        description: role_draft.description,
        permissions: role_draft.permissions,
    };
    assert_eq!(role, expected);

    // Act - Login the Normal user
    app_normal.login_assert().await;

    // Assert - Ensure they have no permissions
    let user = app_normal.core_client.user_info().unwrap();
    assert!(user.permissions.0.is_empty());

    // Act - Log out the normal user
    app_normal
        .core_client
        .logout(no_cb)
        .await
        .expect("failed to receive on rx")
        .expect("failed to log out");

    // Act - Set the role for the normal user
    let req_args = AssignReqArgs {
        username: app_normal.test_user.username.clone().try_into().unwrap(),
        role_id: role.id,
    };
    app_admin
        .core_client
        .assign_role(&req_args, no_cb)
        .await
        .expect("failed to receive on rx")
        .expect("failed to assign role");

    // Act - Login the Normal user
    app_normal.login_assert().await;

    // Assert - Normal user now has the permissions defined
    let user = app_normal.core_client.user_info().unwrap();
    assert_eq!(user.permissions, role.permissions);
}
