use super::{process_json_body, DUMMY_ARGUMENT};
use crate::Client;
use anyhow::{bail, Context as _};
use ewebsock::WsEvent;
use reqwest_cross::fetch_plus;
use reqwest_cross::oneshot;
use reqwest_cross::reqwest;
use tracing::warn;
use wykies_shared::websockets::WebSocketConnection;
use wykies_shared::{
    const_config::path::{PathSpec, PATH_WS_PREFIX},
    token::AuthToken,
};

const WS_CONNECTION_PREFIX: &str = "/ws";

pub trait WakeFn: Fn() + Send + Sync + 'static + Clone {}
impl<T> WakeFn for T where T: Fn() + Send + Sync + 'static + Clone {}

impl Client {
    #[tracing::instrument(skip(wake_up))]
    pub fn ws_connect<F: WakeFn>(
        &self,
        path_spec: PathSpec,
        wake_up: F,
    ) -> oneshot::Receiver<anyhow::Result<WebSocketConnection>> {
        let ws_url = self.ws_url_from(&path_spec);
        let req = self.create_request_builder(path_spec, &DUMMY_ARGUMENT);
        let response_handler = move |resp: reqwest::Result<reqwest::Response>| async {
            do_connect_ws(resp, ws_url, wake_up).await
        };
        fetch_plus(req, response_handler, || {})
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

#[tracing::instrument(ret, err(Debug))]
async fn wait_for_connection_to_open(conn: &mut WebSocketConnection) -> anyhow::Result<()> {
    let event = wait_for_ws_event(conn)
        .await
        .context("any message while waiting for connection to open")?;

    let base_err_msg = "expected first websocket event to be opened but instead got a";

    match event {
        WsEvent::Opened => Ok(()),
        WsEvent::Message(ws_message) => {
            bail!("{base_err_msg} message: {ws_message:?}")
        }
        WsEvent::Error(err_msg) => {
            // Using I'm A Tea Pot as unable to send more detailed error back
            if err_msg.contains("418") {
                // Using I'm a teapot to communicate it's an Unexpected Client as we can only
                // get the status code
                warn!("UnexpectedClient");
                bail!("Server Reported Expected Connection (This may happen sometimes but should not happen very often)")
            } else {
                bail!("{base_err_msg}n error: {err_msg}")
            }
        }
        WsEvent::Closed => {
            bail!("{base_err_msg} Closed event")
        }
    }
}

/// Provides a wrapper when we need to wait on a response in an async context
pub async fn wait_for_ws_event(conn: &mut WebSocketConnection) -> anyhow::Result<WsEvent> {
    // TODO 4: Add a timeout (that's why it returns a result right now)
    loop {
        if let Some(m) = conn.rx.try_recv() {
            break Ok(m);
        } else {
            reqwest_cross::yield_now().await;
        }
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
