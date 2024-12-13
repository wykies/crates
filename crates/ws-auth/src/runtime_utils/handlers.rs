use crate::{AuthTokenManager, WsId};
use actix_web::{dev::ConnectionInfo, web};
use anyhow::Context as _;
use std::sync::Arc;
use wykies_shared::{e500, host_branch::HostId, session::UserSessionInfo, token::AuthToken};

#[tracing::instrument(ret, err(Debug))]
pub async fn get_ws_token(
    auth_manager: web::Data<AuthTokenManager>,
    conn: ConnectionInfo,
    user_info: web::ReqData<UserSessionInfo>,
    ws_id: WsId,
) -> actix_web::Result<web::Json<AuthToken>> {
    let result = AuthToken::new_rand();
    let host_id: HostId = conn
        .try_into()
        .context("failed to get host_id")
        .map_err(e500)?;
    auth_manager.record_token(
        host_id,
        ws_id,
        Arc::new(user_info.into_inner()),
        result.clone(),
    );
    Ok(web::Json(result))
}
