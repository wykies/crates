use std::sync::{Arc, Mutex};

use super::WsId;
use anyhow::{bail, Context as _};
use futures_util::StreamExt as _;
use tokio::time::timeout;
use tracing::{field, warn, Span};
use wykies_shared::{
    const_config::web_socket::{WS_MAX_CONTINUATION_SIZE, WS_MAX_FRAME_SIZE},
    host_branch::HostId,
    session::UserSessionInfo,
    token::AuthToken,
};
use wykies_time::{Seconds, Timestamp};

/// Manages tokens for connecting to websocket endpoints
/// Each token inserted only allows at most once use
/// Tokens are only valid until the record lifetime elapses
#[derive(Debug)]
pub struct AuthTokenManager {
    // ASSUMPTION: small enough scale that a vec is the most efficient implementation
    // This implementation assumes that we only have one instance of the application as it depends
    // on values stored in memory. Would need to be stored in redis if we want it to scale
    // horizontally
    record_lifetime: Seconds,
    auth_records: Mutex<Vec<AuthRecord>>,
}

#[derive(Debug)]
struct AuthRecord {
    timestamp: Timestamp,
    host_id: HostId,
    user_info: Arc<UserSessionInfo>,
    ws_id: WsId,
    token: AuthToken,
}

impl AuthTokenManager {
    #[tracing::instrument(name = "New Auth_Token_Manager")]
    pub fn new(record_lifetime: Seconds) -> Self {
        Self {
            record_lifetime,
            auth_records: Default::default(),
        }
    }

    #[tracing::instrument]
    pub fn record_token(
        &self,
        host_id: HostId,
        ws_id: WsId,
        user_info: Arc<UserSessionInfo>,
        token: AuthToken,
    ) {
        self.purge_stale();
        let timestamp = Timestamp::now();
        let auth_record = AuthRecord {
            timestamp,
            host_id,
            user_info,
            token,
            ws_id,
        };
        self.auth_records
            .lock()
            .expect("mutex poisoned")
            .push(auth_record);
    }

    /// Returns true if at least 1 token is stored for this host
    #[tracing::instrument(ret)]
    pub fn is_expected_host(&self, host_id: &HostId, ws_id: WsId) -> bool {
        self.purge_stale();
        self.auth_records
            .lock()
            .expect("mutex poisoned")
            .iter()
            .any(|rec| &rec.host_id == host_id && rec.ws_id == ws_id)
    }

    /// Validates a token, if validated returns the associated user_info and
    /// removes the token so it may not be reused
    #[tracing::instrument(ret)]
    pub fn validate_token(
        &self,
        host_id: &HostId,
        ws_id: WsId,
        token: &AuthToken,
    ) -> Option<Arc<UserSessionInfo>> {
        self.purge_stale();
        let mut guard = self.auth_records.lock().expect("mutex poisoned");
        let position = guard
            .iter()
            .position(|rec| &rec.host_id == host_id && rec.ws_id == ws_id && &rec.token == token);
        if let Some(index) = position {
            let record = guard.swap_remove(index); // Ensure only used once
            Some(record.user_info)
        } else {
            None
        }
    }

    #[tracing::instrument(fields(len_before=field::Empty, len_after=field::Empty, remove_count=field::Empty))]
    fn purge_stale(&self) {
        let current_timestamp = Timestamp::now();
        let mut guard = self.auth_records.lock().expect("mutex poisoned");
        Span::current().record("len_before", guard.len());
        let len_before = guard.len();
        guard.retain(|rec| {
            let Some(seconds_elapsed) = rec.timestamp.elapsed() else {
                warn!(
                    ?current_timestamp,
                    ?rec.timestamp, "Deleted a auth_record with a timestamp in the future",
                );
                return false;
            };
            self.record_lifetime >= seconds_elapsed
        });
        Span::current().record("len_after", guard.len());
        Span::current().record("remove_count", len_before - guard.len());
    }
}

#[tracing::instrument(err(Debug), skip(msg_stream))]
pub async fn validate_ws_connection(
    msg_stream: actix_ws::MessageStream,
    auth_manager: actix_web::web::Data<AuthTokenManager>,
    client_identifier: &HostId,
    ws_id: WsId,
) -> anyhow::Result<(Arc<UserSessionInfo>, actix_ws::AggregatedMessageStream)> {
    let mut msg_stream = msg_stream
        .max_frame_size(WS_MAX_FRAME_SIZE)
        .aggregate_continuations()
        .max_continuation_size(WS_MAX_CONTINUATION_SIZE);

    match timeout(auth_manager.record_lifetime.into(), msg_stream.next()).await {
        Ok(opt) => match opt {
            Some(res) => match res {
                Ok(msg) => match msg {
                    actix_ws::AggregatedMessage::Text(token) => {
                        let token: AuthToken = token.to_string().into();
                        if let Some(user_info) =
                            auth_manager.validate_token(client_identifier, ws_id, &token)
                        {
                            Ok((user_info, msg_stream))
                        } else {
                            bail!("invalid token received for {client_identifier}: ({token:?})")
                        }
                    }
                    msg => bail!("expected a text message but got {msg:?}"),
                },
                Err(e) => Err(e).context("protocol error received"),
            },
            None => bail!("client stream ended before token was received for {client_identifier}"),
        },
        Err(_) => {
            bail!("timed out waiting for token for {client_identifier}")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use wykies_shared::{
        id::DbId,
        random_string, random_string_def_len,
        uac::{DisplayName, Username},
    };

    use super::*;

    const TEST_RECORD_LIFETIME: Seconds = Seconds::new(1);

    fn new_manager() -> AuthTokenManager {
        AuthTokenManager::new(TEST_RECORD_LIFETIME)
    }

    fn random_host() -> HostId {
        random_string_def_len().try_into().expect(
            "HostId should be long enough to store a valid random string of the recommended length",
        )
    }

    fn new_user() -> Arc<UserSessionInfo> {
        let username = random_string(Username::MAX_LENGTH).try_into().unwrap();
        let display_name = random_string(DisplayName::MAX_LENGTH).try_into().unwrap();
        Arc::new(UserSessionInfo {
            username,
            display_name,
            branch_id: DbId::from(1),
            permissions: Default::default(),
        })
    }

    fn new_token() -> AuthToken {
        AuthToken::new_rand()
    }

    #[test]
    fn valid_token_accepted() {
        let (manager, host_id, user_info, token) =
            (new_manager(), random_host(), new_user(), new_token());
        manager.record_token(
            host_id.clone(),
            WsId::TEST1,
            user_info.clone(),
            token.clone(),
        );
        assert!(manager.is_expected_host(&host_id, WsId::TEST1));
        assert_eq!(
            manager.validate_token(&host_id, WsId::TEST1, &token),
            Some(user_info)
        );
    }

    #[test]
    fn unexpected_host_rejected() {
        let (manager, host_id1, user_info, token) =
            (new_manager(), random_host(), new_user(), new_token());
        let host_id2 = random_host();
        assert!(
            !manager.is_expected_host(&host_id2, WsId::TEST1),
            "not expected when empty"
        );
        manager.record_token(host_id1, WsId::TEST1, user_info, token);
        assert!(
            !manager.is_expected_host(&host_id2, WsId::TEST1),
            "not expected when not empty but not inserted"
        );
    }

    #[test]
    fn stale_token_not_allowed() {
        let (manager, host_id, user_info, token) =
            (new_manager(), random_host(), new_user(), new_token());
        manager.record_token(
            host_id.clone(),
            WsId::TEST1,
            user_info.clone(),
            token.clone(),
        );
        assert!(manager.is_expected_host(&host_id, WsId::TEST1));

        // Sleep for token to get stale
        std::thread::sleep(TEST_RECORD_LIFETIME.into());
        std::thread::sleep(Duration::from_secs(1)); // Add 1 more second to ensure it's stale

        // Old token rejected
        assert!(!manager.is_expected_host(&host_id, WsId::TEST1));
        assert!(manager
            .validate_token(&host_id, WsId::TEST1, &token)
            .is_none());

        // Insert another token for the same host
        manager.record_token(host_id.clone(), WsId::TEST1, user_info, new_token());

        // Ensure old token is still rejected
        assert!(
            manager.is_expected_host(&host_id, WsId::TEST1),
            "host should be valid now we just inserted a new record for it"
        );
        assert!(
            manager
                .validate_token(&host_id, WsId::TEST1, &token)
                .is_none(),
            "token should still be invalid we inserted a new token"
        );
    }

    #[test]
    fn multiple_tokens_allowed_for_host() {
        let manager = new_manager();
        let host_id = random_host();
        let user_info = new_user();
        let token1 = new_token();
        let token2 = new_token();

        manager.record_token(
            host_id.clone(),
            WsId::TEST1,
            user_info.clone(),
            token1.clone(),
        );
        manager.record_token(
            host_id.clone(),
            WsId::TEST1,
            user_info.clone(),
            token2.clone(),
        );

        assert!(manager.is_expected_host(&host_id, WsId::TEST1));

        assert_eq!(
            manager.validate_token(&host_id, WsId::TEST1, &token1),
            Some(user_info.clone())
        );
        assert_eq!(
            manager.validate_token(&host_id, WsId::TEST1, &token2),
            Some(user_info)
        );
    }

    #[test]
    fn invalid_token_rejected() {
        let manager = new_manager();
        let host_id = random_host();
        let user_info = new_user();
        let token1 = new_token();
        let token2 = new_token();

        manager.record_token(host_id.clone(), WsId::TEST1, user_info, token1.clone());

        assert!(manager.is_expected_host(&host_id, WsId::TEST1));
        assert!(manager
            .validate_token(&host_id, WsId::TEST1, &token2)
            .is_none());
    }

    #[test]
    fn token_unable_to_be_reused() {
        let (manager, host_id, user_info, token) =
            (new_manager(), random_host(), new_user(), new_token());
        manager.record_token(
            host_id.clone(),
            WsId::TEST1,
            user_info.clone(),
            token.clone(),
        );

        assert!(manager.is_expected_host(&host_id, WsId::TEST1));
        assert_eq!(
            manager.validate_token(&host_id, WsId::TEST1, &token),
            Some(user_info)
        );

        assert!(!manager.is_expected_host(&host_id, WsId::TEST1));
        assert!(manager
            .validate_token(&host_id, WsId::TEST1, &token)
            .is_none());
    }

    #[test]
    fn ws_type_must_match() {
        // TODO 3: Implement test
    }
}
