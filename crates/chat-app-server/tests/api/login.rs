use crate::helpers::{no_cb, spawn_app};
use std::sync::{Arc, Mutex};
use wykies_client_core::LoginOutcome;
use wykies_shared::{req_args::LoginReqArgs, uac::AuthError};

#[tokio::test]
async fn login_failure_invalid_user() {
    // Arrange
    let app = spawn_app().await;
    let login_args = LoginReqArgs::new(
        "random-username".to_string(),
        "random-password".to_string().into(),
    );

    // Act
    let outcome = app.core_client.login(login_args, no_cb).await.unwrap();

    // Assert
    assert_eq!(
        outcome.unwrap_err().to_string(),
        AuthError::InvalidUserOrPassword.to_string()
    );
}

#[tokio::test]
async fn login_failure_invalid_password() {
    // Arrange
    let app = spawn_app().await;
    let login_args = app
        .test_user
        .login_args()
        .password("random-password".to_string().into());

    // Act
    let outcome = app.core_client.login(login_args, no_cb).await.unwrap();

    // Assert
    assert_eq!(
        outcome.unwrap_err().to_string(),
        AuthError::InvalidUserOrPassword.to_string()
    );
}

#[tokio::test]
async fn login_failure_not_enabled() {
    // Arrange
    let app = spawn_app().await;
    app.test_user.disable_in_db(&app).await;

    // Act
    let outcome = app.login().await;

    // Assert
    assert_eq!(
        outcome.unwrap_err().to_string(),
        AuthError::NotEnabled.to_string()
    );
}

#[tokio::test]
async fn login_logout_round_trip() {
    // Arrange
    let app = spawn_app().await;

    // Assert - Ensure not logged in
    assert!(
        !app.is_logged_in().await,
        "should not be logged in before logging in"
    );

    // Act - Login
    let login_outcome = app.login().await.unwrap();

    // Assert - Login successful and user info stored
    assert_eq!(login_outcome, LoginOutcome::ForcePasswordChange);
    assert_eq!(
        app.core_client.user_info().unwrap().username.as_ref(),
        app.test_user.username
    );

    // Assert - Ensure we are logged in
    assert!(
        app.is_logged_in().await,
        "should be logged in after logging in"
    );

    // Act - Logout
    app.logout_assert().await;

    // Assert - Ensure we are not logged in
    assert!(
        !app.is_logged_in().await,
        "should not be logged in after logging out"
    );
}

#[tokio::test]
async fn ensure_call_back_is_run() {
    // Arrange
    let app = spawn_app().await;
    let test_flag = Arc::new(Mutex::new(false));
    let test_flag_clone = Arc::clone(&test_flag);

    // Act
    assert!(app
        .core_client
        .login(app.test_user.login_args(), move || {
            *test_flag_clone.lock().unwrap() = true;
        })
        .await
        .expect("failed to receive from rx")
        .expect("failed to get result of login")
        .is_any_success());

    // Assert
    assert!(*test_flag.lock().unwrap(), "flag was not flipped");
}

#[tokio::test]
/// Check that flag is respected even without any attempts
async fn login_user_locked_out_no_attempts() {
    // Arrange
    let app = spawn_app().await;
    app.test_user.set_locked_out_in_db(&app, true).await;

    // Act
    let outcome = app.login().await;

    // Assert
    assert_eq!(
        outcome.unwrap_err().to_string(),
        AuthError::LockedOut.to_string()
    );
}

#[tokio::test]
async fn ensure_user_gets_locked_out() {
    // Arrange
    let app = spawn_app().await;
    let login_args = app
        .test_user
        .login_args()
        .password("random-password".to_string().into());

    // Assert - Ensure not logged in
    assert!(
        !app.is_logged_in().await,
        "should not be logged in before logging in"
    );

    // Act - Repeat attempting login one less than the limit so they should still
    // not be locked out
    let login_attempt_limit = app.login_attempt_limit;
    for _ in 1..login_attempt_limit {
        // Attempt login
        let outcome = app
            .core_client
            .login(login_args.clone(), no_cb)
            .await
            .unwrap();

        // Ensure not locked out
        assert_eq!(
            outcome.unwrap_err().to_string(),
            AuthError::InvalidUserOrPassword.to_string()
        );
    }

    // Attempt login again which should trigger the lockout
    let outcome = app.core_client.login(login_args, no_cb).await.unwrap();

    // Assert - User is locked out
    assert_eq!(
        outcome.unwrap_err().to_string(),
        AuthError::LockedOut.to_string()
    );

    // Act - Attempt login again with correct password
    let outcome = app.login().await;

    // Assert - User is still locked out
    assert_eq!(
        outcome.unwrap_err().to_string(),
        AuthError::LockedOut.to_string()
    );
}

#[tokio::test]
async fn ensure_locked_out_is_reset() {
    // Arrange
    let app = spawn_app().await;
    let login_args = app
        .test_user
        .login_args()
        .password("random-password".to_string().into());

    // Assert - Ensure not logged in
    assert!(
        !app.is_logged_in().await,
        "should not be logged in before logging in"
    );

    // Act - Repeat attempting login one less than the limit so they should still
    // not be locked out
    let login_attempt_limit = app.login_attempt_limit;
    for _ in 1..login_attempt_limit {
        // Attempt login
        let outcome = app
            .core_client
            .login(login_args.clone(), no_cb)
            .await
            .unwrap();

        // Ensure not locked out
        assert_eq!(
            outcome.unwrap_err().to_string(),
            AuthError::InvalidUserOrPassword.to_string()
        );
    }

    // Login Successfully which should reset the count
    app.login_assert().await;

    // Assert - User is logged in
    assert!(app.is_logged_in().await);

    // Act - Log out
    app.logout_assert().await;

    // Assert - user is logged out
    assert!(!app.is_logged_in().await);

    // Act - Repeat attempting login one less than the limit so they should still
    // not be locked out
    let login_attempt_limit = app.login_attempt_limit;
    for _ in 1..login_attempt_limit {
        // Attempt login
        let outcome = app
            .core_client
            .login(login_args.clone(), no_cb)
            .await
            .unwrap();

        // Ensure not locked out
        assert_eq!(
            outcome.unwrap_err().to_string(),
            AuthError::InvalidUserOrPassword.to_string()
        );
    }
}
