//! Happy path tested in other modules just testing authentication here

use ewebsock::WsEvent;
use wykies_client_core::ws_expose_test::{self, EXPOSE_TEST_DUMMY_ARGUMENT};
use wykies_shared::{const_config::path::PATH_WS_TOKEN_CHAT, token::AuthToken};

use crate::helpers::{no_cb, spawn_app, wait_for_message};

#[tokio::test]
async fn rejected_without_requesting_token() {
    // Arrange
    let app = spawn_app().await;
    app.login_assert().await;
    let ws_url = app.core_client.expose_test_ws_url_from(&PATH_WS_TOKEN_CHAT);

    // Try to connect
    let conn = ws_expose_test::initiate_ws_connection(ws_url, no_cb).unwrap();

    // Get response
    let response = wait_for_message(&conn.rx, false).await.unwrap();

    // Assert
    assert_eq!(
        format!("{:?}", response),
        format!(
            "{:?}",
            WsEvent::Error("HTTP error: 400 Bad Request".to_string())
        )
    );
}

#[tokio::test]
async fn fails_to_connect_without_correct_token() {
    // Arrange
    let app = spawn_app().await;
    app.login_assert().await;
    let ws_url = app.core_client.expose_test_ws_url_from(&PATH_WS_TOKEN_CHAT);

    // Request token
    let _token: AuthToken = app
        .core_client
        .expose_test_send_request_expect_json(
            PATH_WS_TOKEN_CHAT,
            &EXPOSE_TEST_DUMMY_ARGUMENT,
            no_cb,
        )
        .await
        .expect("failed to get msg from rx")
        .expect("failed to extract token");

    // Initiate connection
    let mut conn = ws_expose_test::initiate_ws_connection(ws_url, no_cb).unwrap();

    // Wait for connection to be opened
    ws_expose_test::wait_for_connection_to_open(&mut conn)
        .await
        .unwrap();

    // Send wrong token
    let token = AuthToken::new_rand();
    conn.tx.send(token.into());

    // Get response
    let response = wait_for_message(&conn.rx, false).await.unwrap();

    // Assert - Assert that `Closed` is received. Note anything but `Closed` is an
    // error including `Ping`
    assert_eq!(format!("{:?}", response), format!("{:?}", WsEvent::Closed));
}
