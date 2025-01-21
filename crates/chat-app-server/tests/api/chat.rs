use crate::helpers::{no_cb, spawn_app, wait_for_message};
use ewebsock::{WsEvent, WsMessage};
use plugin_chat::{
    consts::{CHAT_HISTORY_RECENT_CAPACITY, CHAT_HISTORY_REQUEST_SIZE},
    ChatIM, ChatImText, ChatMsg, ChatMsgsHistory, ChatUser, InitialStateBody, ReqHistoryBody,
};
use pretty_assertions::{assert_eq, assert_ne};
use std::time::Duration;
use tokio::time::sleep;
use wykies_server_test_helper::expect_ok;
use wykies_shared::{const_config::path::PATH_WS_TOKEN_CHAT, uac::Username};
use wykies_time::Timestamp;

#[tokio::test]
async fn sent_messages_received() {
    // Arrange
    let app = spawn_app().await;
    app.login_assert().await;
    let mut conn1 = expect_ok!(app.core_client.ws_connect(PATH_WS_TOKEN_CHAT, no_cb));
    let conn2 = expect_ok!(app.core_client.ws_connect(PATH_WS_TOKEN_CHAT, no_cb));
    let author: Username = app.test_user.username.clone().try_into().unwrap();
    let expected_im = ChatMsg::IM(ChatIM {
        author: author.clone(),
        timestamp: Timestamp::now(),
        content: "test message".try_into().unwrap(),
    });
    let msg = WsMessage::Text(serde_json::to_string(&expected_im).unwrap());
    let chat_user = ChatUser::new(author);
    let expected_initial_state = WsEvent::Message(WsMessage::Text(
        serde_json::to_string(&ChatMsg::InitialState(InitialStateBody {
            connected_users: vec![(chat_user, 2)],
            history: ChatMsgsHistory { ims: Vec::new() },
        }))
        .unwrap(),
    ));

    // Act - Wait for initial message
    let actual_initial_state = wait_for_message(&conn2.rx, true)
        .await
        .expect("failed to receive initial state");

    // Assert
    assert_eq!(
        format!("{actual_initial_state:?}"),
        format!("{expected_initial_state:?}")
    );

    // Act
    conn1.tx.send(msg.clone());

    let incoming = wait_for_message(&conn2.rx, true)
        .await
        .expect("failed to receive message");

    let mut actual: ChatMsg = match incoming {
        WsEvent::Message(WsMessage::Text(text)) => serde_json::from_str(&text).unwrap(),
        other => panic!("Actual:   {other:?}\nExpected: {:?}", WsEvent::Message(msg)),
    };

    // Prevent test from being flakey as server might change the time stamp
    if let (ChatMsg::IM(actual), ChatMsg::IM(expected)) = (&mut actual, &expected_im) {
        actual.timestamp = expected.timestamp
    }

    // Assert
    assert_eq!(actual, expected_im);
}

#[tokio::test]
async fn connect_to_chat() {
    // Arrange
    let app = spawn_app().await;
    app.login_assert().await;

    // Connect Websocket
    expect_ok!(app.core_client.ws_connect(PATH_WS_TOKEN_CHAT, no_cb));
}

#[tokio::test]
async fn chat_initial_buffered_history() {
    // Arrange
    let app = spawn_app().await;
    app.login_assert().await;
    let author: Username = app.test_user.username.clone().try_into().unwrap();
    let expected_ims_texts: Vec<ChatImText> = (1..9)
        .map(|i| format!("Message #{i}").try_into().unwrap())
        .collect();

    // Act - Connect Websocket
    let mut conn = expect_ok!(app.core_client.ws_connect(PATH_WS_TOKEN_CHAT, no_cb));

    // Act - Send messages
    for im in expected_ims_texts.iter() {
        let msg = ChatMsg::IM(ChatIM {
            author: author.clone(),
            timestamp: Timestamp::now(),
            content: im.clone(),
        });
        let msg = WsMessage::Text(serde_json::to_string(&msg).unwrap());
        conn.tx.send(msg);
    }
    tokio::time::sleep(std::time::Duration::from_millis(100)).await; //Wait for message to be sent before dropping sender

    // Act - Reconnect to see if messages are included in the history
    conn = expect_ok!(app.core_client.ws_connect(PATH_WS_TOKEN_CHAT, no_cb));

    // Act - Wait for initial state message
    let incoming = wait_for_message(&conn.rx, true)
        .await
        .expect("failed to receive message");

    // Assert
    match incoming {
        WsEvent::Message(WsMessage::Text(text)) => {
            let msg: ChatMsg = serde_json::from_str(&text).unwrap();
            let ims = match msg {
                ChatMsg::InitialState(InitialStateBody {
                    history: ChatMsgsHistory { ims, .. },
                    ..
                }) => ims,
                other => panic!("expected initial state but got: {other:?}"),
            };
            assert!(
                ims.iter().all(|im| im.author == author),
                "unexpected author in: {ims:?}"
            );
            let actual_im_texts: Vec<ChatImText> = ims.into_iter().map(|im| im.content).collect();
            assert_eq!(actual_im_texts, expected_ims_texts)
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[tokio::test]
async fn chat_overflowing_server_history_buffer() {
    // Arrange
    let app = spawn_app().await;
    app.login_assert().await;
    let author: Username = app.test_user.username.clone().try_into().unwrap();
    const MSGS_SENT: u64 = 2 * CHAT_HISTORY_RECENT_CAPACITY as u64;
    let expected_ims_texts: Vec<ChatImText> = (0..MSGS_SENT)
        .map(|i| format!("{i:0>4}").try_into().unwrap())
        .collect();

    // Act - Connect Websocket
    let mut conn = expect_ok!(app.core_client.ws_connect(PATH_WS_TOKEN_CHAT, no_cb));

    // Act - Send messages
    let sleep_interval = CHAT_HISTORY_REQUEST_SIZE as usize / 2;
    for (count, im) in expected_ims_texts.iter().enumerate() {
        let msg = ChatMsg::IM(ChatIM {
            author: author.clone(),
            timestamp: Timestamp::now(),
            content: im.clone(),
        });
        let msg = WsMessage::Text(serde_json::to_string(&msg).unwrap());
        conn.tx.send(msg);
        if count % sleep_interval == 0 {
            // Sleep to ensure all messages do not have the same timestamp
            sleep(Duration::from_secs(1)).await;
        }
    }
    tokio::time::sleep(std::time::Duration::from_millis(
        // Wait 3 millis for each message to be sent before dropping (expected to be way too much)
        3 * MSGS_SENT,
    ))
    .await;

    // Act - Reconnect to see if messages are included in the history
    conn = expect_ok!(app.core_client.ws_connect(PATH_WS_TOKEN_CHAT, no_cb));

    // Act - Wait for initial state message
    let incoming = wait_for_message(&conn.rx, true)
        .await
        .expect("failed to receive message");

    // Act - Extract history from initial message
    let mut history = match incoming {
        WsEvent::Message(WsMessage::Text(text)) => {
            let msg: ChatMsg = serde_json::from_str(&text).unwrap();
            match msg {
                ChatMsg::InitialState(InitialStateBody { history, .. }) => history,
                other => panic!("expected initial state but got: {other:?}"),
            }
        }
        other => panic!("unexpected event: {other:?}"),
    };

    // Act - Get other history
    // Works based on the assumption that each request might only contain half of
    // the records that are new because they are sent so quickly
    let request_count =
        ((MSGS_SENT as usize - history.len()) / CHAT_HISTORY_REQUEST_SIZE as usize) * 2 + 1;
    let qty = CHAT_HISTORY_REQUEST_SIZE;
    for i in 0..request_count {
        let current_earliest_timestamp = history.earliest_timestamp_or_now();
        let chat_msg = ChatMsg::ReqHistory(ReqHistoryBody {
            qty,
            latest_timestamp: current_earliest_timestamp,
        });
        conn.tx
            .send(WsMessage::Text(serde_json::to_string(&chat_msg).unwrap()));
        let incoming = wait_for_message(&conn.rx, true)
            .await
            .expect("failed to receive message");
        let more_history = match incoming {
            WsEvent::Message(WsMessage::Text(text)) => {
                let msg: ChatMsg = serde_json::from_str(&text).unwrap();
                match msg {
                    ChatMsg::RespHistory(history) => history,
                    other => panic!("expected Response to History Request but got: {other:?}"),
                }
            }
            other => panic!("unexpected event: {other:?}"),
        };

        let is_empty = more_history.is_empty();
        history.prepend_other(more_history).unwrap();
        if is_empty {
            assert_ne!(i, 0, "No history was retrieved");
            // Should be i and not (i-1) because it is 0 based so if it only got on the
            // first one then i will be 1
            println!("All history retrieved in {i} requests");
            break;
        }
    }

    // Apply this sort for testing purposes because we don't care much about the
    // order of messages sent at the same time in prod
    history
        .ims
        .sort_by_cached_key(|x| (x.timestamp, x.content.to_string()));

    // Assert - Ensure we got the messages we were expecting
    let actual: Vec<ChatImText> = history.ims.into_iter().map(|x| x.content).collect();
    println!(
        "Expected: {} messages and got {}",
        expected_ims_texts.len(),
        actual.len()
    );
    assert_eq!(actual, expected_ims_texts);
}
