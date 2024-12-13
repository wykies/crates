use crate::session_state::TypedSession;
use actix_web::HttpResponse;
use wykies_shared::e500;

pub async fn log_out(session: TypedSession) -> actix_web::Result<HttpResponse> {
    if session.get_user_info().map_err(e500)?.is_none() {
        // This should never happen for this function call as the user should always be
        // logged in but may happen if the user is not logged in
        Err(actix_web::error::ErrorInternalServerError(
            "Should be logged in to get here but unable to retrieve the username",
        ))
    } else {
        session.log_out();
        Ok(HttpResponse::Ok().finish())
    }
}
