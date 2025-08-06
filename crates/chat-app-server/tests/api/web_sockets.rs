//! Happy path tested in other modules just testing authentication here

use ewebsock::WsEvent;
use wykies_client_core::DUMMY_ARGUMENT;
use wykies_server_test_helper::TEST_MSG_WAIT_TIMEOUT;
use wykies_shared::{
    const_config::path::PATH_WS_TOKEN_CHAT, token::AuthToken, websockets::WsConnTxRx,
};

use crate::helpers::{no_cb, spawn_app};

#[tokio::test]
async fn rejected_without_requesting_token() {
    // Arrange
    let app = spawn_app().await;
    app.login_assert().await;
    let ws_url = app
        .core_client
        .expose_internal_ws_url_from(&PATH_WS_TOKEN_CHAT);

    // Try to connect
    let mut conn = WsConnTxRx::initiate_connection(ws_url, no_cb).unwrap();

    // Get response
    let response = conn
        .recv_with_timeout_ignoring_ping(TEST_MSG_WAIT_TIMEOUT)
        .await
        .unwrap();

    // Assert
    assert_eq!(
        format!("{response:?}"),
        format!(
            "{:?}",
            WsEvent::Error("HTTP error: 418 I'm a teapot".to_string())
        )
    );
}

#[tokio::test]
async fn fails_to_connect_without_correct_token() {
    // Arrange
    let app = spawn_app().await;
    app.login_assert().await;
    let ws_url = app
        .core_client
        .expose_internal_ws_url_from(&PATH_WS_TOKEN_CHAT);

    // Request token
    let _token: AuthToken = app
        .core_client
        .expose_internal_send_request_expect_json(PATH_WS_TOKEN_CHAT, &DUMMY_ARGUMENT)
        .await
        .expect("failed to get msg from rx")
        .expect("failed to extract token");

    // Initiate connection
    let mut conn = WsConnTxRx::initiate_connection(ws_url, no_cb).unwrap();

    // Wait for connection to be opened
    conn.wait_for_connection_to_open(TEST_MSG_WAIT_TIMEOUT)
        .await
        .unwrap();

    // Send wrong token
    let token = AuthToken::new_rand();
    conn.send(token.into());

    // Get response
    let response = conn
        .recv_with_timeout_ignoring_ping(TEST_MSG_WAIT_TIMEOUT)
        .await
        .unwrap();

    // Assert - Assert that `Closed` is received. Note anything but `Closed` is an
    // error including `Ping`
    assert_eq!(format!("{response:?}"), format!("{:?}", WsEvent::Closed));
}
