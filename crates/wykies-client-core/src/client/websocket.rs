use std::fmt::Debug;

use anyhow::{bail, Context as _};
use ewebsock::WsEvent;
use futures::channel::oneshot;
use wykies_shared::{
    const_config::path::{PathSpec, PATH_WS_PREFIX},
    token::AuthToken,
};

use crate::Client;

use super::{process_json_body, DUMMY_ARGUMENT};

const WS_CONNECTION_PREFIX: &str = "/ws";

pub struct WebSocketConnection {
    pub tx: ewebsock::WsSender,
    pub rx: ewebsock::WsReceiver,
}

pub trait WakeFn: Fn() + Send + Sync + 'static {}
impl<T> WakeFn for T where T: Fn() + Send + Sync + 'static {}

impl Client {
    #[tracing::instrument(skip(wake_up))]
    pub fn ws_connect<F: WakeFn>(
        &self,
        path_spec: PathSpec,
        wake_up: F,
    ) -> oneshot::Receiver<anyhow::Result<WebSocketConnection>> {
        let (tx, rx) = oneshot::channel();
        let ws_url = self.ws_url_from(&path_spec);
        let on_done = move |resp: reqwest::Result<reqwest::Response>| async {
            let result = do_connect_ws(resp, ws_url, wake_up).await;
            tx.send(result).expect("failed to send oneshot msg");
        };
        self.initiate_request(path_spec, &DUMMY_ARGUMENT, on_done);
        rx
    }

    /// Appends `path` onto the base websocket url
    ///
    /// # Panic
    ///
    /// Panics if the server_address does not start with "http"
    #[tracing::instrument(ret)]
    fn ws_url_from(&self, path_spec: &PathSpec) -> String {
        assert_eq!(&path_spec.path[..PATH_WS_PREFIX.len()], PATH_WS_PREFIX);
        let suffix = &path_spec.path[PATH_WS_PREFIX.len()..];
        let mut result = "ws".to_string();
        {
            let guard = self.inner.lock().expect("client-core mutex poisoned");
            let server_address = &guard.server_address;
            assert!(server_address.starts_with("http"));
            result.push_str(&server_address[4..]);
        }
        result.push_str(WS_CONNECTION_PREFIX);
        result.push_str(suffix);
        result
    }

    #[cfg(feature = "expose_internal")]
    pub fn expose_internal_ws_url_from(&self, path_spec: &PathSpec) -> String {
        self.ws_url_from(path_spec)
    }
}

#[tracing::instrument(skip(wake_up))]
async fn do_connect_ws<F: WakeFn>(
    response: reqwest::Result<reqwest::Response>,
    ws_url: String,
    wake_up: F,
) -> anyhow::Result<WebSocketConnection> {
    // Get token from response passed in
    let token = extract_token(response).await?;

    // Initiate connection
    let mut result = initiate_ws_connection(ws_url, wake_up)?;

    // Wait for connection to complete before sending token
    wait_for_connection_to_open(&mut result).await?;

    // Send token
    result.tx.send(token.into());

    Ok(result)
}

async fn wait_for_connection_to_open(conn: &mut WebSocketConnection) -> anyhow::Result<()> {
    let event = loop {
        if let Some(m) = conn.rx.try_recv() {
            break m;
        } else {
            reqwest_cross::yield_now().await;
        }
    };
    if matches!(event, WsEvent::Opened) {
        Ok(())
    } else {
        bail!("expected first event to be opened but got {event:?}")
    }
}

async fn extract_token(response: reqwest::Result<reqwest::Response>) -> anyhow::Result<AuthToken> {
    process_json_body(response).await
}

fn initiate_ws_connection<F>(ws_url: String, wake_up: F) -> anyhow::Result<WebSocketConnection>
where
    F: WakeFn,
{
    let (tx, rx) = ewebsock::connect_with_wakeup(ws_url, Default::default(), wake_up)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("failed to connect web socket")?;
    Ok(WebSocketConnection { tx, rx })
}

impl Debug for WebSocketConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebSocketConnection {{ .. }} ")
    }
}

#[cfg(feature = "expose_internal")]
pub mod expose_internal {

    use super::{WakeFn, WebSocketConnection};

    pub fn initiate_ws_connection<F>(
        ws_url: String,
        wake_up: F,
    ) -> anyhow::Result<WebSocketConnection>
    where
        F: WakeFn,
    {
        super::initiate_ws_connection(ws_url, wake_up)
    }

    pub async fn wait_for_connection_to_open(conn: &mut WebSocketConnection) -> anyhow::Result<()> {
        super::wait_for_connection_to_open(conn).await
    }
}
