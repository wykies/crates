use secrecy::SecretString;
use uuid::Uuid;
use wykies_client_core::LoginOutcome;
use wykies_shared::{
    req_args::{
        api::admin::user::{NewUserReqArgs, PasswordResetReqArgs},
        LoginReqArgs,
    },
    uac::{ResetPasswordError, UserMetadata, UserMetadataDiff, Username},
};

use crate::helpers::{no_cb, spawn_app};

#[tokio::test]
async fn list_users_and_roles() {
    // Arrange
    let app = spawn_app().await.create_admin_user().await;
    app.login_assert().await;

    // Act
    let actual = app
        .core_client
        .list_users_and_roles(no_cb)
        .await
        .expect("failed to receive on rx")
        .expect("failed to extract result");

    // Assert
    insta::assert_json_snapshot!(actual, {
        ".users[].username" => "[value varies]",
        ".users[].pass_change_date" => "[date]"
    });
}

#[tokio::test]
async fn user() {
    // Arrange
    let app = spawn_app().await.create_admin_user().await;
    app.login_assert().await;

    // Act
    let actual = app
        .core_client
        .get_user(app.test_user.username.clone().try_into().unwrap(), no_cb)
        .await
        .expect("failed to receive on rx")
        .expect("failed to extract result");

    // Assert
    insta::assert_json_snapshot!(actual, {
        ".username" => "[value varies]",
        ".pass_change_date" => "[date]"
    });
}

#[tokio::test]
async fn user_update_display_name() {
    common_update_user_test(|mut user| {
        user.display_name = "Edited Name".to_string().try_into().unwrap();
        user
    })
    .await
}

#[tokio::test]
async fn user_update_force_pass_change() {
    common_update_user_test(|mut user| {
        user.force_pass_change = false;
        user
    })
    .await
}

#[tokio::test]
async fn user_update_assigned_role() {
    common_update_user_test(|mut user| {
        user.assigned_role = None;
        user
    })
    .await
}

#[tokio::test]
async fn user_update_enabled() {
    common_update_user_test(|mut user| {
        user.enabled = false;
        user
    })
    .await
}

#[tokio::test]
async fn user_update_locked_out() {
    common_update_user_test(|mut user| {
        user.locked_out = true;
        user.failed_attempts = 10;
        user
    })
    .await
}

#[tokio::test]
async fn user_update_all() {
    common_update_user_test(|mut user| {
        user.display_name = "All Changed".to_string().try_into().unwrap();
        user.assigned_role = Some(1.into());
        user.enabled = false;
        user.force_pass_change = false;
        user.locked_out = true;
        user.failed_attempts = 10;
        user
    })
    .await
}

async fn common_update_user_test(f: impl FnOnce(UserMetadata) -> UserMetadata) {
    // Arrange
    let app = spawn_app().await.create_admin_user().await;
    app.login_assert().await;

    // Arrange -- Get User from DB
    let original_user = app
        .core_client
        .get_user(app.test_user.username.clone().try_into().unwrap(), no_cb)
        .await
        .expect("failed to receive on rx")
        .expect("failed to extract result");

    // Arrange -- Create modified user
    let edited_user = f(original_user.clone());

    // Arrange -- Create Diff
    let diff = UserMetadataDiff::from_diff(&original_user, &edited_user)
        .expect("username must match")
        .expect("no difference found");

    // Act -- Push change
    app.core_client
        .update_user(diff, no_cb)
        .await
        .expect("failed to receive on rx")
        .expect("failed to extract result");

    // Act -- Get updated user
    let actual = app
        .core_client
        .get_user(app.test_user.username.clone().try_into().unwrap(), no_cb)
        .await
        .expect("failed to receive on rx")
        .expect("failed to extract result");

    // Assert
    assert_eq!(actual, edited_user);
}

#[tokio::test]
async fn new_user() {
    // Arrange
    let app = spawn_app().await.create_admin_user().await;
    app.login_assert().await;
    let username: Username = "New User".to_string().try_into().unwrap();
    let password: SecretString = "a test password".to_string().into();
    let req_args = NewUserReqArgs {
        username: username.clone(),
        display_name: "Display New".to_string().try_into().unwrap(),
        password: password.clone(),
        assigned_role: None,
    };

    // Act
    app.core_client
        .new_user(req_args.clone(), no_cb)
        .await
        .expect("failed to receive on rx")
        .expect("failed to extract result");
    // TODO 4: Add macro to add the expects as there isn't much value in copying
    // this every time. Needs to be macro and not a function to capture the location
    // of the panic in the code.
    let actual = app
        .core_client
        .get_user(username.clone(), no_cb)
        .await
        .expect("failed to receive on rx")
        .expect("failed to extract result");

    // Assert
    let expected = UserMetadata {
        username: req_args.username,
        display_name: req_args.display_name,
        force_pass_change: true,
        assigned_role: req_args.assigned_role,
        enabled: true,
        locked_out: false,
        failed_attempts: 0,
        pass_change_date: chrono::Utc::now().date_naive(),
    };
    assert_eq!(actual, expected);

    // Arrange -- Logout to test logging in as new user
    app.logout_assert().await;
    let login_args = LoginReqArgs::new(username, password);

    // Act
    let outcome = app
        .core_client
        .login(login_args, no_cb)
        .await
        .expect("failed to receive on rx")
        .expect("failed to extract result");

    // Assert
    assert_eq!(outcome, LoginOutcome::ForcePasswordChange);
}

#[tokio::test]
async fn password_reset_normal() {
    // Arrange
    let mut app_normal = spawn_app().await;
    let app_admin = app_normal.create_admin_user().await;
    let new_password = Uuid::new_v4().to_string();
    let password_reset_req_args = PasswordResetReqArgs {
        username: app_normal.test_user.username.clone().try_into().unwrap(),
        new_password: new_password.clone().into(),
    };
    app_admin.login_assert().await;

    // Act - Change password
    app_admin
        .core_client
        .reset_password(password_reset_req_args, no_cb)
        .await
        .expect("failed to receive on rx")
        .expect("failed to extract result");

    // Act - Login using the new password
    app_normal.test_user.password = new_password;
    let login_outcome = app_normal.login().await.unwrap();

    // Assert - Login succeeded
    assert_eq!(login_outcome, LoginOutcome::ForcePasswordChange);
}

#[tokio::test]
async fn password_reset_blocked_same_user() {
    // Arrange
    let app = spawn_app().await.create_admin_user().await;
    let args = PasswordResetReqArgs {
        username: app.test_user.username.clone().try_into().unwrap(),
        new_password: Uuid::new_v4().to_string().into(),
    };
    app.login_assert().await;

    // Act
    let actual = app
        .core_client
        .reset_password(args, no_cb)
        .await
        .expect("failed to receive on rx")
        .expect_err("failed to extract error");

    // Assert
    assert_eq!(
        actual.to_string(),
        ResetPasswordError::NoResetOwnPassword.to_string()
    );
}
