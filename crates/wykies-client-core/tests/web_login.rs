//! IMPORTANT!!!
//! A server must be started up on localhost separately (Will not work in CI due
//! to IPv6). Only intended for local testing. Expects a "new" database (For
//! instance expects the user to still be required to do a password change).
//! From the folder for the server crate run `cargo run --features disable-cors`
//! to start the server. Then from the folder "crates/wykies-client-core" run
//! one of the following to execute the tests
//! - `wasm-pack test --headless --firefox`
//! - `wasm-pack test --headless --chrome`
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_bindgen_test::wasm_bindgen_test_configure;
use wykies_client_core::{Client, LoginOutcome};
use wykies_shared::const_config::path::PATH_WS_TOKEN_CHAT;
use wykies_shared::req_args::LoginReqArgs;

wasm_bindgen_test_configure!(run_in_browser);
fn main() {
    #[wasm_bindgen_test]
    async fn login_logout_round_trip() {
        // Arrange
        // ASSUMING SERVER HAS BEEN STARTED (See module docs comment)
        let client = Client::default();
        let login_args = LoginReqArgs::new_with_branch(
            "seed_admin".to_string(),
            "f".to_string().into(),
            1.into(),
        );

        // Assert - Ensure not logged in
        assert!(
            !is_logged_in(&client).await,
            "should not be logged in before logging in"
        );

        // Act - Login
        let login_outcome = client.login(login_args.clone(), no_cb).await.unwrap();

        // Assert - Login successful and user info stored
        assert_eq!(
            login_outcome
                .expect("IMPORTANT!!! ensure server is started properly see module doc comment"),
            LoginOutcome::ForcePasswordChange
        );
        assert_eq!(
            client.user_info().unwrap().username.as_ref(),
            &login_args.username
        );
        // Unable to go further likely because the sensitive headers are being
        // sanitized at some point. This includes the cookies.
        // Still keeping this test to check for basic functionality and testing
        // for deadlocks
    }
}

async fn is_logged_in(client: &Client) -> bool {
    // Also tests if able to establish a websocket connection but this was the
    // simplest alternative that didn't need any permissions
    client
        .ws_connect(PATH_WS_TOKEN_CHAT, no_cb)
        .await
        .expect("failed to receive on rx")
        .is_ok()
}

fn no_cb() {}
