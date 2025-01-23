use super::{process_json_body, DUMMY_ARGUMENT};
use crate::Client;
use reqwest_cross::{fetch_plus, oneshot, reqwest};
use tracing::warn;
use wykies_shared::{
    const_config::path::{PathSpec, PATH_WS_PREFIX},
    token::AuthToken,
    websockets::{WakeFn, WsConnTxRx},
};

const WS_CONNECTION_PREFIX: &str = "/ws";

impl Client {
    #[tracing::instrument(skip(wake_up))]
    pub fn ws_connect<F: WakeFn>(
        &self,
        path_spec: PathSpec,
        wake_up: F,
    ) -> oneshot::Receiver<anyhow::Result<WsConnTxRx>> {
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
) -> anyhow::Result<WsConnTxRx> {
    // Get token from response passed in
    let token = extract_token(response).await?;

    // Initiate connection
    WsConnTxRx::initiate_connection_with_auth(token, ws_url, wake_up).await
}

async fn extract_token(response: reqwest::Result<reqwest::Response>) -> anyhow::Result<AuthToken> {
    process_json_body(response).await
}
