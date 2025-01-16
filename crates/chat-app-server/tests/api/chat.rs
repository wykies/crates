use crate::helpers::{no_cb, spawn_app, wait_for_message};
use ewebsock::{WsEvent, WsMessage};
use plugin_chat::{ChatIM, ChatImText, ChatMsg, ChatUser, InitialStateBody, RespHistoryBody};
use wykies_shared::{const_config::path::PATH_WS_TOKEN_CHAT, uac::Username};
use wykies_time::Timestamp;

#[tokio::test]
async fn sent_messages_received() {
    // Arrange
    let app = spawn_app().await;
    app.login_assert().await;
    let mut conn1 = app
        .core_client
        .ws_connect(PATH_WS_TOKEN_CHAT, no_cb)
        .await
        .expect("failed to receive on rx")
        .expect("connection result was not ok");
    let conn2 = app
        .core_client
        .ws_connect(PATH_WS_TOKEN_CHAT, no_cb)
        .await
        .expect("failed to receive on rx")
        .expect("connection result was not ok");
    let author: Username = app.test_user.username.clone().try_into().unwrap();
    let expected_im = ChatMsg::IM(ChatIM {
        author: author.clone(),
        timestamp: Timestamp::now(),
        content: "test message".try_into().unwrap(),
    });
    let msg = WsMessage::Text(serde_json::to_string(&expected_im).unwrap());
    let chat_user = ChatUser::new(author);
    let expected_initial_state = WsEvent::Message(WsMessage::Text(
        serde_json::to_string(&ChatMsg::InitialState(InitialStateBody::new(
            vec![(chat_user, 2)],
            RespHistoryBody::new(Vec::new()),
        )))
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
    app.core_client
        .ws_connect(PATH_WS_TOKEN_CHAT, no_cb)
        .await
        .expect("failed to receive on rx")
        .expect("connection result was not ok");
}

#[tokio::test]
async fn load_history() {
    // Arrange
    let app = spawn_app().await;
    app.login_assert().await;
    let author: Username = app.test_user.username.clone().try_into().unwrap();
    let expected_ims_texts: Vec<ChatImText> = (1..9)
        .map(|i| format!("Message #{i}").try_into().unwrap())
        .collect();

    // Act - Connect Websocket
    let mut conn = app
        .core_client
        .ws_connect(PATH_WS_TOKEN_CHAT, no_cb)
        .await
        .expect("failed to receive on rx")
        .expect("connection result was not ok");

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

    // Act - Reconnect to see if messages are sent in the history
    conn = app
        .core_client
        .ws_connect(PATH_WS_TOKEN_CHAT, no_cb)
        .await
        .expect("failed to receive on rx")
        .expect("connection result was not ok");

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
                    history: RespHistoryBody { ims, .. },
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

// TODO 4: Add a test for load_more chat messages (Would need to overflow the
//          server cache buffer)
// TODO 4: Add test for saving to DB (overflow save buffer
//          qty as time would take too long)
