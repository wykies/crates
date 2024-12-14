use crate::helpers::{no_cb, spawn_app};
use uuid::Uuid;
use wykies_shared::{
    errors::NotLoggedInError, req_args::api::ChangePasswordReqArgs, uac::ChangePasswordError,
};

#[tokio::test]
async fn you_must_be_logged_in_to_change_your_password() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let args = ChangePasswordReqArgs {
        current_password: Uuid::new_v4().to_string().into(),
        new_password: new_password.clone().to_string().into(),
        new_password_check: new_password.to_string().into(),
    };

    // Act
    let actual = app.core_client.change_password(&args, no_cb).await.unwrap();

    // Assert
    assert_eq!(
        actual.unwrap_err().to_string(),
        NotLoggedInError.to_string()
    );
}

#[tokio::test]
async fn new_password_fields_must_match() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let another_new_password = Uuid::new_v4().to_string();

    // Act - Login
    app.login_assert().await;

    // Act - Try to change password
    let actual = app
        .core_client
        .change_password(
            &ChangePasswordReqArgs {
                current_password: app.test_user.password.into(),
                new_password: new_password.clone().into(),
                new_password_check: another_new_password.into(),
            },
            no_cb,
        )
        .await
        .unwrap();

    // Assert - Password mismatch rejected
    assert_eq!(
        actual.unwrap_err().to_string(),
        ChangePasswordError::PasswordsDoNotMatch.to_string()
    );
}

#[tokio::test]
async fn current_password_must_be_valid() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let wrong_password = Uuid::new_v4().to_string();

    // Act - Login
    app.login_assert().await;

    // Act - Try to change password
    let actual = app
        .core_client
        .change_password(
            &ChangePasswordReqArgs {
                current_password: wrong_password.into(),
                new_password: new_password.clone().into(),
                new_password_check: new_password.into(),
            },
            no_cb,
        )
        .await
        .unwrap();

    // Assert - Rejected wrong current password
    assert_eq!(
        actual.unwrap_err().to_string(),
        ChangePasswordError::CurrentPasswordWrong(
            wykies_shared::uac::AuthError::InvalidUserOrPassword,
        )
        .to_string()
    );
}

#[tokio::test]
async fn changing_password_works() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    // Act - Login
    app.login_assert().await;

    // Act - Change password
    let actual = app
        .core_client
        .change_password(
            &ChangePasswordReqArgs {
                current_password: app.test_user.password.clone().into(),
                new_password: new_password.clone().into(),
                new_password_check: new_password.clone().into(),
            },
            no_cb,
        )
        .await
        .unwrap();

    // Assert
    actual.unwrap();

    // Act - Logout
    app.logout_assert().await;

    // Act - Login using the new password
    let login_outcome = app
        .core_client
        .login(
            app.test_user.login_args().password(new_password.into()),
            no_cb,
        )
        .await
        .unwrap();

    // Assert - Login succeeded
    assert!(login_outcome.unwrap().is_any_success());
}
