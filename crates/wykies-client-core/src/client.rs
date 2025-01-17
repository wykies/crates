use anyhow::{anyhow, bail, Context};
use reqwest_cross::reqwest::{self, Method, RequestBuilder, StatusCode};
use reqwest_cross::{fetch, fetch_plus, oneshot};
use secrecy::ExposeSecret as _;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};
use wykies_shared::uac::LoginResponse;
use wykies_shared::{
    branch::Branch,
    const_config::path::{PathSpec, PATH_BRANCHES, PATH_HEALTH_CHECK, PATH_LOGIN},
    req_args::LoginReqArgs,
    uac::UserInfo,
};

pub mod api;
pub mod websocket;

pub const DUMMY_ARGUMENT: &[(&str, &str)] = &[("", "")];

#[derive(Debug, Clone)]
pub struct Client {
    api_client: reqwest::Client,
    inner: Arc<Mutex<ClientInner>>,
}

#[derive(Debug)]
struct ClientInner {
    server_address: String,
    user_info: Option<Arc<UserInfo>>,
}

impl Default for Client {
    fn default() -> Self {
        // TODO 3: Load url from server config into binary at compile time
        // TODO 3: Add test to ensure URL starts with "http" so when we replace "http"
        //          with "ws"  it will not be a problem. Ignore the following s as both
        //          would need it. Both https and wss.
        if cfg!(debug_assertions) {
            Self::new("http://localhost:8789".to_string())
        } else {
            Self::new("https://chat-demo-umon.shuttle.app".to_string())
        }
    }
}

#[must_use]
#[derive(Debug, PartialEq, Eq)]
pub enum LoginOutcome {
    Success,
    ForcePasswordChange,
    RetryWithBranchSet,
}

impl LoginOutcome {
    /// Returns `true` if the login outcome is
    /// [`Success`] or [`ForcePasswordChange`]
    ///
    /// [`Success`]: LoginOutcome::Success
    /// [`ForcePasswordChange`]: LoginOutcome::ForcePasswordChange
    #[must_use]
    pub fn is_any_success(&self) -> bool {
        matches!(self, Self::Success) || matches!(self, Self::ForcePasswordChange)
    }
}

impl ClientInner {
    #[tracing::instrument]
    fn new(server_address: String) -> Self {
        Self {
            server_address,
            user_info: None,
        }
    }
}

impl Client {
    #[tracing::instrument(name = "NEW CLIENT-CORE")]
    pub fn new(server_address: String) -> Self {
        let api_client_builder = reqwest::Client::builder();
        #[cfg(not(target_arch = "wasm32"))]
        let api_client_builder = api_client_builder.cookie_store(true);
        let api_client = api_client_builder
            .build()
            .expect("Unable to create reqwest client");
        Self {
            api_client,
            inner: Arc::new(Mutex::new(ClientInner::new(server_address))),
        }
    }

    #[tracing::instrument]
    pub fn get_branches(&self) -> oneshot::Receiver<anyhow::Result<Vec<Branch>>> {
        self.send_request_expect_json(PATH_BRANCHES, &DUMMY_ARGUMENT)
    }

    #[tracing::instrument]
    pub fn login(&self, args: LoginReqArgs) -> oneshot::Receiver<anyhow::Result<LoginOutcome>> {
        let args = serde_json::json!({
            "username": args.username,
            "password": args.password.expose_secret(),
            "branch_to_set": args.branch_to_set,
        });
        let req = self.create_request_builder(PATH_LOGIN, &args);
        let client = self.clone();
        let response_handler = move |resp: reqwest::Result<reqwest::Response>| async {
            process_login(resp, client).await
        };
        fetch_plus(req, response_handler, || {})
    }

    #[tracing::instrument]
    pub fn health_check(&self) -> oneshot::Receiver<anyhow::Result<()>> {
        self.send_request_expect_empty(PATH_HEALTH_CHECK, &DUMMY_ARGUMENT)
    }

    #[tracing::instrument(skip(args))]
    // WARNING: Must skip args as it my contain sensitive info and "safe" versions
    // would usually already be logged by the caller
    fn create_request_builder<T>(&self, path_spec: PathSpec, args: &T) -> RequestBuilder
    where
        T: serde::Serialize + Debug,
    {
        let is_get_method = path_spec.method == Method::GET;
        let request = self
            .api_client
            .request(path_spec.method, self.path_to_url(path_spec.path));
        if is_get_method {
            request.query(&args)
        } else {
            request.json(&args)
        }
    }

    fn send_request_expect_json<T, U>(
        &self,
        path_spec: PathSpec,
        args: &T,
    ) -> oneshot::Receiver<anyhow::Result<U>>
    where
        T: serde::Serialize + std::fmt::Debug,
        U: Send + std::fmt::Debug + serde::de::DeserializeOwned + 'static,
    {
        let req = self.create_request_builder(path_spec, args);
        let response_handler =
            move |resp: reqwest::Result<reqwest::Response>| async { process_json_body(resp).await };
        fetch_plus(req, response_handler, || {})
    }

    #[cfg(feature = "expose_internal")]
    pub fn expose_internal_send_request_expect_json<T, U>(
        &self,
        path_spec: PathSpec,
        args: &T,
    ) -> oneshot::Receiver<anyhow::Result<U>>
    where
        T: serde::Serialize + std::fmt::Debug,
        U: Send + std::fmt::Debug + serde::de::DeserializeOwned + 'static,
    {
        self.send_request_expect_json(path_spec, args)
    }

    #[cfg(feature = "expose_internal")]
    pub fn expose_internal_send_request_expect_empty<T>(
        &self,
        path_spec: PathSpec,
        args: &T,
    ) -> oneshot::Receiver<anyhow::Result<()>>
    where
        T: serde::Serialize + std::fmt::Debug,
    {
        self.send_request_expect_empty(path_spec, args)
    }

    fn send_request_expect_empty<T>(
        &self,
        path_spec: PathSpec,
        args: &T,
    ) -> oneshot::Receiver<anyhow::Result<()>>
    where
        T: serde::Serialize + std::fmt::Debug,
    {
        let req = self.create_request_builder(path_spec, args);
        let response_handler =
            move |resp: reqwest::Result<reqwest::Response>| async { process_empty(resp).await };
        fetch_plus(req, response_handler, || {})
    }

    /// Sends the request but only logs the response
    fn send_request_no_wait<T>(&self, path_spec: PathSpec, args: &T)
    where
        T: serde::Serialize + std::fmt::Debug,
    {
        let req = self.create_request_builder(path_spec, args);
        fetch(req, |resp| async {
            match resp {
                Ok(resp) => info!(
                    resp_status_code = resp.status().to_string(),
                    "ignored response received and it was Ok"
                ),
                Err(err_msg) => warn!(?err_msg, "ignored response received and it was an Err"),
            }
        });
    }

    #[tracing::instrument(ret)]
    fn path_to_url(&self, path: &str) -> String {
        format!(
            "{}{path}",
            &self
                .inner
                .lock()
                .expect("failed to unlock client mutex")
                .server_address
        )
    }

    pub fn user_info(&self) -> Option<Arc<UserInfo>> {
        self.inner.lock().expect("mutex poisoned").user_info.clone()
    }

    pub fn is_logged_in(&self) -> bool {
        self.inner
            .lock()
            .expect("mutex poisoned")
            .user_info
            .is_some()
    }
}

#[tracing::instrument(ret, err(Debug))]
async fn process_empty(response: reqwest::Result<reqwest::Response>) -> anyhow::Result<()> {
    let (response, status) = extract_response(response)?;
    if status == StatusCode::OK {
        Ok(())
    } else {
        Err(handle_error(response).await)
    }
}

#[tracing::instrument(ret, err(Debug))]
async fn process_json_body<T>(response: reqwest::Result<reqwest::Response>) -> anyhow::Result<T>
where
    T: Debug + serde::de::DeserializeOwned,
{
    let (response, status) = extract_response(response)?;
    match status {
        StatusCode::OK => Ok(response
            .json()
            .await
            .context("failed to parse result as json")?),
        _ => Err(handle_error(response).await),
    }
}

#[tracing::instrument(ret, err(Debug))]
async fn process_login(
    response: reqwest::Result<reqwest::Response>,
    client: Client,
) -> anyhow::Result<LoginOutcome> {
    let (response, status) = extract_response(response)?;
    match status {
        StatusCode::OK => {
            let login_response: LoginResponse = response
                .json()
                .await
                .context("failed to parse result as json")?;
            let (result, user_info) = match login_response {
                LoginResponse::Success(user_info) => (LoginOutcome::Success, user_info),
                LoginResponse::SuccessForcePassChange(user_info) => {
                    (LoginOutcome::ForcePasswordChange, user_info)
                }
            };
            client.inner.lock().expect("mutex poisoned").user_info = Some(Arc::new(user_info));
            Ok(result)
        }
        StatusCode::FAILED_DEPENDENCY => Ok(LoginOutcome::RetryWithBranchSet),
        _ => Err(handle_error(response).await),
    }
}

#[tracing::instrument(ret)]
async fn handle_error(response: reqwest::Response) -> anyhow::Error {
    let status = response.status();
    debug_assert!(
        !status.is_success(),
        "this is supposed to be an error, right? Status code is: {status}"
    );
    let Ok(body) = response.text().await else {
        return anyhow!("failed to get response body");
    };
    if body.is_empty() {
        anyhow!("request failed with status code: {status} and no body")
    } else {
        anyhow!("{body}")
    }
}

/// Provides a way to standardize the error message
#[tracing::instrument(ret, err(Debug))]
fn extract_response(
    response: reqwest::Result<reqwest::Response>,
) -> anyhow::Result<(reqwest::Response, StatusCode)> {
    let response = match response {
        Ok(x) => x,
        Err(err_msg) => {
            #[cfg(target_arch = "wasm32")]
            let is_connected = "NOT AVAILABLE ON WASM";
            #[cfg(not(target_arch = "wasm32"))]
            let is_connected = err_msg.is_connect();

            warn!(
                ?err_msg,
                is_body = ?err_msg.is_body(),
                is_builder = ?err_msg.is_builder(),
                ?is_connected,
                is_decode = ?err_msg.is_decode(),
                is_redirect = ?err_msg.is_redirect(),
                is_request = ?err_msg.is_request(),
                is_status = ?err_msg.is_status(),
                is_timeout = ?err_msg.is_timeout(),
                status = ?err_msg.status(),
                url = ?err_msg.url(),
                "reqwest::Error is: {err_msg}"
            );
            let custom_msg = match &err_msg {
                #[cfg(not(target_arch = "wasm32"))]
                e if e.is_connect() => "Server Not Reachable",
                _ => "Request Failed",
            };
            bail!("{custom_msg}: {err_msg}")
        }
    };
    let status = response.status();
    Ok((response, status))
}
