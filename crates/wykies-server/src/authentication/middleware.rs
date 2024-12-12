use crate::error_wrappers::e500;
use crate::session_state::TypedSession;
use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    middleware::Next,
    FromRequest, HttpMessage,
};
use tracing::info;
use wykies_shared::{
    errors::NotLoggedInError,
    session::UserSessionInfo,
    uac::{get_required_permissions, PermissionsError},
};

/// Ensures the user is logged in and has the required permissions to get to the
/// endpoint The endpoint may do further permission checking based on the
/// content of the request but top level endpoint permission validation happens
/// at this point
#[tracing::instrument(skip(next))]
pub async fn validate_user_access(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let session = {
        let (http_request, payload) = req.parts_mut();
        TypedSession::from_request(http_request, payload).await
    }?;

    // TODO 6: Might need to add a keep alive from the client to hold the connection
    // open if the user is not communicating with the server but is using the
    // program

    match session.get_user_info().map_err(e500)? {
        Some(user_info) => {
            check_permissions(&req, &user_info).await?;
            info!("Validated request for {:?}", user_info.username.as_ref());
            // TODO 4: Add check that client identifier still matches what is saved
            // otherwise log and reject connection
            req.extensions_mut().insert(user_info);
            next.call(req).await
        }
        None => Err(NotLoggedInError.into()),
    }
}

/// Checks that the user has the required permissions to access the endpoint.
/// If no permissions are found for the endpoint a 503 error is returned (See
/// [`wykies_shared::uac::get_required_permissions`])
#[tracing::instrument]
async fn check_permissions(
    req: &ServiceRequest,
    user_info: &UserSessionInfo,
) -> Result<(), PermissionsError> {
    let Some(required_perms) = get_required_permissions(req.path()) else {
        return Err(PermissionsError::PathNotFound(req.path().to_string()));
    };
    if user_info.permissions.includes(required_perms) {
        Ok(())
    } else {
        Err(PermissionsError::MissingPermissions(
            required_perms.to_vec(),
        ))
    }
}
