use anyhow::{anyhow, Context};
use closure_traits::{ChannelCallBack, ChannelCallBackOutput};
use futures::channel::oneshot;
use reqwest::{Method, StatusCode};
use secrecy::ExposeSecret as _;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use tracing::info;
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
        Self::new("http://localhost:8789".to_string())
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
        let api_client = reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .expect("Unable to create reqwest client");
        Self {
            api_client,
            inner: Arc::new(Mutex::new(ClientInner::new(server_address))),
        }
    }

    #[tracing::instrument(skip(ui_notify))]
    pub fn get_branches<F>(&self, ui_notify: F) -> oneshot::Receiver<anyhow::Result<Vec<Branch>>>
    where
        F: UiCallBack,
    {
        self.send_request_expect_json(PATH_BRANCHES, &DUMMY_ARGUMENT, ui_notify)
    }

    #[tracing::instrument(skip(ui_notify))]
    pub fn login<F: UiCallBack>(
        &self,
        args: LoginReqArgs,
        ui_notify: F,
    ) -> oneshot::Receiver<anyhow::Result<LoginOutcome>> {
        let (tx, rx) = oneshot::channel();
        let args = serde_json::json!({
            "username": args.username,
            "password": args.password.expose_secret(),
            "branch_to_set": args.branch_to_set,
        });
        let client = self.clone();
        let on_done = move |resp: reqwest::Result<reqwest::Response>| async {
            let msg = process_login(resp, client).await;
            tx.send(msg).expect("failed to send oneshot msg");
            ui_notify();
        };

        self.initiate_request(PATH_LOGIN, &args, on_done);
        rx
    }

    #[tracing::instrument(skip(ui_notify))]
    pub fn health_check<F>(&self, ui_notify: F) -> oneshot::Receiver<anyhow::Result<()>>
    where
        F: UiCallBack,
    {
        self.send_request_expect_empty(PATH_HEALTH_CHECK, &DUMMY_ARGUMENT, ui_notify)
    }

    #[tracing::instrument(skip(args, on_done))]
    // WARNING: Must skip args as it my contain sensitive info and "safe" versions
    // would usually already be logged by the caller
    fn initiate_request<T, F, O>(&self, path_spec: PathSpec, args: &T, on_done: F)
    where
        T: serde::Serialize + Debug,
        F: ChannelCallBack<O>,
        O: ChannelCallBackOutput,
    {
        let is_get_method = path_spec.method == Method::GET;
        let mut request = self
            .api_client
            .request(path_spec.method, self.path_to_url(path_spec.path));
        request = if is_get_method {
            request.query(&args)
        } else {
            request.json(&args)
        };
        reqwest_cross::fetch(request, on_done)
    }

    fn send_request_expect_json<F, T, U>(
        &self,
        path_spec: PathSpec,
        args: &T,
        ui_notify: F,
    ) -> oneshot::Receiver<anyhow::Result<U>>
    where
        T: serde::Serialize + std::fmt::Debug,
        F: UiCallBack,
        U: Send + std::fmt::Debug + serde::de::DeserializeOwned + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let on_done = move |resp: reqwest::Result<reqwest::Response>| async {
            let msg = process_json_body(resp).await;
            tx.send(msg).expect("failed to send oneshot msg");
            ui_notify();
        };
        self.initiate_request(path_spec, args, on_done);
        rx
    }

    #[cfg(feature = "expose_internal")]
    pub fn expose_internal_send_request_expect_json<F, T, U>(
        &self,
        path_spec: PathSpec,
        args: &T,
        ui_notify: F,
    ) -> oneshot::Receiver<anyhow::Result<U>>
    where
        T: serde::Serialize + std::fmt::Debug,
        F: UiCallBack,
        U: Send + std::fmt::Debug + serde::de::DeserializeOwned + 'static,
    {
        self.send_request_expect_json(path_spec, args, ui_notify)
    }

    fn send_request_expect_empty<F, T>(
        &self,
        path_spec: PathSpec,
        args: &T,
        ui_notify: F,
    ) -> oneshot::Receiver<anyhow::Result<()>>
    where
        T: serde::Serialize + std::fmt::Debug,
        F: UiCallBack,
    {
        let (tx, rx) = oneshot::channel();
        let on_done = move |resp: reqwest::Result<reqwest::Response>| async {
            let msg = process_empty(resp).await;
            tx.send(msg).expect("failed to send oneshot msg");
            ui_notify();
        };
        self.initiate_request(path_spec, args, on_done);
        rx
    }

    fn send_request_no_wait<T>(&self, path_spec: PathSpec, args: &T)
    where
        T: serde::Serialize + std::fmt::Debug,
    {
        self.initiate_request(path_spec, args, |_| async {});
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
    if response.is_err() {
        info!("Response is err: {:#?}", response);
    }
    let response = response.context("failed to send request")?;
    let status = response.status();
    Ok((response, status))
}

pub trait UiCallBack: 'static + Send + FnOnce() {}
impl<T> UiCallBack for T where T: 'static + Send + FnOnce() {}

#[cfg(not(target_arch = "wasm32"))]
pub mod closure_traits {
    pub trait ChannelCallBack<O>:
        'static + Send + FnOnce(reqwest::Result<reqwest::Response>) -> O
    {
    }
    impl<T, O> ChannelCallBack<O> for T where
        T: 'static + Send + FnOnce(reqwest::Result<reqwest::Response>) -> O
    {
    }
    pub trait ChannelCallBackOutput: futures::Future<Output = ()> + Send {}
    impl<T> ChannelCallBackOutput for T where T: futures::Future<Output = ()> + Send {}
}

#[cfg(target_arch = "wasm32")]
pub mod closure_traits {
    pub trait ChannelCallBack<O>:
        'static + FnOnce(reqwest::Result<reqwest::Response>) -> O
    {
    }
    impl<T, O> ChannelCallBack<O> for T where
        T: 'static + FnOnce(reqwest::Result<reqwest::Response>) -> O
    {
    }
    pub trait ChannelCallBackOutput: futures::Future<Output = ()> {}
    impl<T> ChannelCallBackOutput for T where T: futures::Future<Output = ()> {}
}
