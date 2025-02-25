use wykies_client_core::LoginOutcome;
use wykies_server_test_helper::expect_ok;
use wykies_shared::{
    branch::BranchDraft, host_branch::HostBranchPair, req_args::api::host_branch::LookupReqArgs,
    uac::AuthError,
};

use crate::helpers::{spawn_app, spawn_app_without_host_branch_stored, TestApp};

#[tokio::test]
async fn set_host_branch_pair() {
    // Arrange
    let app_admin = spawn_app().await.create_admin_user().await;
    let branch_draft = BranchDraft {
        name: "test name".try_into().unwrap(),
        short_name: "te".try_into().unwrap(),
    };

    // Act - Login the admin
    app_admin.login_assert().await;

    // Arrange - Create Branch
    let branch_id = expect_ok!(app_admin.core_client.create_branch(&branch_draft));
    let mut host_branch_pair = HostBranchPair {
        host_id: "Host name or IP".to_string().try_into().unwrap(),
        branch_id,
    };

    // Create New Pair
    send_request_and_verify_response(&app_admin, &host_branch_pair).await;

    // Do Same Pair Again
    send_request_and_verify_response(&app_admin, &host_branch_pair).await;

    // Act - Create new branch
    let branch_draft = BranchDraft {
        name: "test name2".try_into().unwrap(),
        short_name: "t2".try_into().unwrap(),
    };
    let branch_id = expect_ok!(app_admin.core_client.create_branch(&branch_draft));
    host_branch_pair.branch_id = branch_id;

    // Update Host to New Branch
    send_request_and_verify_response(&app_admin, &host_branch_pair).await;
}

async fn send_request_and_verify_response(app: &TestApp, pair: &HostBranchPair) {
    // Act - Set Pair (Create / Update)
    expect_ok!(app.core_client.create_host_branch_pair(pair));

    // Act - Retrieve current list of pairs
    let pairs = expect_ok!(app.core_client.get_list_host_branch_pairs());

    // Assert - Verify Pair was created
    assert!(
        pairs.contains(pair),
        "actual: {pairs:?}, expected it to include: {pair:?}"
    );
}

#[tokio::test]
async fn host_branch_pair_lookup() {
    // Arrange
    let app_admin = spawn_app().await.create_admin_user().await;
    let branch_draft = BranchDraft {
        name: "test name".try_into().unwrap(),
        short_name: "te".try_into().unwrap(),
    };

    // Act - Login the admin
    app_admin.login_assert().await;

    // Arrange - Create Branch
    let branch_id = expect_ok!(app_admin.core_client.create_branch(&branch_draft));
    let host_branch_pair = HostBranchPair {
        host_id: "Host name or IP".to_string().try_into().unwrap(),
        branch_id,
    };

    // Act - Do lookup
    let args = LookupReqArgs {
        host_id: host_branch_pair.host_id.clone(),
    };
    let actual = expect_ok!(app_admin.core_client.get_host_branch_pair(&args));

    // Assert - Ensure not found
    assert_eq!(actual, None);

    // Create New Pair
    send_request_and_verify_response(&app_admin, &host_branch_pair).await;

    // Act - Do lookup
    let arg = LookupReqArgs {
        host_id: host_branch_pair.host_id.clone(),
    };
    let actual = expect_ok!(app_admin.core_client.get_host_branch_pair(&arg));

    // Assert - Found
    assert_eq!(actual, Some(host_branch_pair.branch_id));
}

#[tokio::test]
async fn ensure_branch_only_changes_if_not_set() {
    // Arrange
    let app_admin = spawn_app().await.create_admin_user().await;

    // Arrange - Create 2nd branch and logout to test setting it
    app_admin.login_assert().await;
    let body = BranchDraft {
        name: "second branch".try_into().unwrap(),
        short_name: "se".try_into().unwrap(),
    };
    let new_branch_id = expect_ok!(app_admin.core_client.create_branch(&body));
    app_admin.logout_assert().await;

    // Act - Login and request branch is changed
    let rx = app_admin.core_client.login(
        app_admin
            .test_user
            .login_args()
            .branch_to_set(Some(new_branch_id)),
    );
    assert!(expect_ok!(rx).is_any_success());

    // Act - Get current branch set
    let curr_branch_id = expect_ok!(app_admin.core_client.get_host_branch_pair(&LookupReqArgs {
        host_id: app_admin.host_branch_pair.host_id.clone(),
    }))
    .expect("expected pair to exist");

    // Assert - Confirm branch has not changed
    assert_ne!(curr_branch_id, new_branch_id);
    assert_eq!(curr_branch_id, app_admin.host_branch_pair.branch_id);
}

#[tokio::test]
async fn ensure_branch_not_set_without_permissions() {
    // Arrange - Setup without branch assigned
    let app = spawn_app_without_host_branch_stored().await;

    // Act - Login without requesting branch be set
    let actual = app.login().await;

    // Assert - Correct error returned
    assert_eq!(
        actual.unwrap_err().to_string(),
        AuthError::BranchNotSetAndUnableToSet {
            client_identifier: app.host_branch_pair.host_id.clone(),
        }
        .to_string()
    );

    // Act - Login again and attempt to set branch
    let actual = app
        .core_client
        .login(
            app.test_user
                .login_args()
                .branch_to_set(Some(app.host_branch_pair.branch_id)),
        )
        .await
        .unwrap();

    // Assert - Correct error returned
    assert_eq!(
        actual.unwrap_err().to_string(),
        AuthError::BranchNotSetAndUnableToSet {
            client_identifier: app.host_branch_pair.host_id.clone(),
        }
        .to_string()
    );

    // Assert - Ensure user is not logged in
    assert!(!app.is_logged_in().await);
}

#[tokio::test]
async fn ensure_branch_can_be_set_with_permissions() {
    // Arrange - Setup without branch assigned
    let app_admin = spawn_app_without_host_branch_stored()
        .await
        .create_admin_user()
        .await;

    // Act - Login without requesting branch be set
    let actual = app_admin.login().await;

    // Assert - Correct error returned
    assert_eq!(actual.unwrap(), LoginOutcome::RetryWithBranchSet);

    // Act - Login again and set branch
    let rx = app_admin.core_client.login(
        app_admin
            .test_user
            .login_args()
            .branch_to_set(Some(app_admin.host_branch_pair.branch_id)),
    );
    assert!(expect_ok!(rx).is_any_success());

    // Act - Get current branch set
    let actual = expect_ok!(app_admin.core_client.get_host_branch_pair(&LookupReqArgs {
        host_id: app_admin.host_branch_pair.host_id.clone(),
    }));

    // Assert - Check set to expected branch
    assert_eq!(actual, Some(app_admin.host_branch_pair.branch_id));

    // Act - Log out
    app_admin.logout_assert().await;

    // Act - Log back in
    app_admin.login_assert().await;

    // Act - Get current branch set
    let actual = expect_ok!(app_admin.core_client.get_host_branch_pair(&LookupReqArgs {
        host_id: app_admin.host_branch_pair.host_id.clone(),
    }));

    // Assert - Branch is still set
    assert_eq!(actual, Some(app_admin.host_branch_pair.branch_id));
}
