use wykies_shared::branch::{Branch, BranchDraft};

use crate::helpers::spawn_app;

#[tokio::test]
async fn create_branch() {
    // Arrange
    let app_admin = spawn_app().await.create_admin_user().await;
    let branch_draft = BranchDraft {
        name: "test name".try_into().unwrap(),
        short_name: "te".try_into().unwrap(),
    };

    // Act - Login the admin
    app_admin.login_assert().await;

    // Act - Create Branch
    let branch_id = app_admin
        .core_client
        .create_branch(&branch_draft)
        .await
        .expect("failed to get msg from rx")
        .expect("failed to extract branch id from result");

    // Assert - Verify branch was created
    let branches = app_admin
        .core_client
        .get_branches()
        .await
        .expect("failed to get msg from rx")
        .expect("failed to extract branches from result");
    let actual = branches.into_iter().find(|x| x.id == branch_id).unwrap();
    let expected = Branch {
        id: branch_id,
        name: branch_draft.name,
        short_name: branch_draft.short_name,
    };
    assert_eq!(actual, expected);
}
