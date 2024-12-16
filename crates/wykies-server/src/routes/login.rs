use super::{execute_chained_handler, set_host_branch_pair};
use crate::{
    authentication::{validate_credentials, AuthUserInfo, Credentials, LoginAttemptLimit},
    routes::host_branch_pair_lookup,
    session_state::TypedSession,
};
use actix_web::{dev::ConnectionInfo, web, HttpResponse};
use anyhow::{anyhow, Context};
use wykies_shared::db_types::DbPool;
use wykies_shared::session::UserSessionInfo;
use wykies_shared::{
    const_config::path::{PATH_API_ADMIN_HOSTBRANCH_SET, PATH_API_HOSTBRANCH_LOOKUP},
    host_branch::{HostBranchPair, HostId},
    id::DbId,
    req_args::{api::admin::host_branch, LoginReqArgs},
    uac::{AuthError, LoginResponse},
};

/// Provides a way for users to create a login session
///
/// - Only successful responses should return a 200 and errors should either by
///   401 or 500
/// - A successful login provides a cookie to access authenticated routes
/// - A login can fail for various reasons and should provide a suitable error
///   message
/// - Sessions should automatically timeout after a period of time
#[tracing::instrument(
    ret,
    err(Debug),
    skip(req_args, pool, session),
    fields(username=tracing::field::Empty)
)]
pub async fn login(
    conn: ConnectionInfo,
    web::Json(req_args): web::Json<LoginReqArgs>,
    pool: web::Data<DbPool>,
    login_attempt_limit: web::Data<LoginAttemptLimit>,
    session: TypedSession,
) -> Result<HttpResponse, AuthError> {
    let credentials = Credentials {
        username: req_args.username,
        password: req_args.password,
    };

    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    let auth_user_info = validate_credentials(credentials, &pool, &login_attempt_limit).await?;
    let set_user_branch_result =
        set_user_branch(&pool, auth_user_info, conn, req_args.branch_to_set).await;
    if set_user_branch_result
        .as_ref()
        .is_err_and(|x| x.is_branch_not_set_resend())
    {
        // Special case return for when the request should be resent and include the
        // branch that should be set
        return Ok(
            HttpResponse::FailedDependency().body(set_user_branch_result.unwrap_err().to_string())
        );
    }
    let login_response = set_user_branch_result?;
    session.renew();
    match &login_response {
        LoginResponse::Success(user_info) | LoginResponse::SuccessForcePassChange(user_info) => {
            session
                .insert_user_info(UserSessionInfo {
                    username: user_info.username.clone(),
                    display_name: user_info.display_name.clone(),
                    branch_id: user_info.branch_id,
                    permissions: user_info.permissions.clone(),
                })
                .context("session update failed")?;
        }
    }
    Ok(HttpResponse::Ok().json(login_response))
}

#[tracing::instrument(skip(pool))]
async fn set_user_branch(
    pool: &DbPool,
    auth_user_info: AuthUserInfo,
    conn: ConnectionInfo,
    branch_to_set: Option<DbId>,
) -> Result<LoginResponse, AuthError> {
    // Extract Client Host Identifier
    let client_identifier: HostId = conn.try_into().context("failed to get host_id")?;

    // Lookup DB for Client Host Identifier
    let lookup_result = execute_chained_handler(
        PATH_API_HOSTBRANCH_LOOKUP.path,
        &auth_user_info.permissions,
        || {
            host_branch_pair_lookup(
                web::Data::new(pool.clone()),
                web::Query(host_branch::LookupReqArgs {
                    host_id: client_identifier.clone(),
                }),
            )
        },
    )?;
    let branch_id: DbId = match lookup_result.await {
        Ok(web::Json(looked_up_id)) => match looked_up_id {
            Some(id) => id,
            None => {
                // No ID found in the DB for this client identifier

                // Check if user has permissions to set branch and branch provided to be set
                let does_user_have_permission = auth_user_info
                    .permissions
                    .is_allowed_access(PATH_API_ADMIN_HOSTBRANCH_SET.path)?;
                match (does_user_have_permission, branch_to_set) {
                    (false, _) => {
                        return Err(AuthError::BranchNotSetAndUnableToSet { client_identifier })
                    }
                    (true, None) => {
                        return Err(AuthError::BranchNotSetResend { client_identifier })
                    }
                    (true, Some(branch_to_set)) => {
                        // Set Branch as per user request
                        if let Err(e) = set_host_branch_pair(
                            web::Data::new(pool.clone()),
                            web::Json(HostBranchPair {
                                host_id: client_identifier,
                                branch_id: branch_to_set,
                            }),
                        )
                        .await
                        {
                            return Err(AuthError::UnexpectedError(anyhow!(
                                "error setting host branch pair: {e:?}"
                            )));
                        }
                        branch_to_set
                    }
                }
            }
        },
        Err(e) => {
            return Err(AuthError::UnexpectedError(anyhow!(
                "host branch pair lookup failed with error: {e:?}"
            )));
        }
    };

    // Return LoginResponse
    Ok(auth_user_info.into_login_response(branch_id)?)
}
