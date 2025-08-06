use crate::authentication::{Credentials, LoginAttemptLimit, validate_credentials};
use actix_web::{HttpResponse, web};
use secrecy::ExposeSecret as _;
use wykies_shared::{
    db_types::DbPool,
    req_args::api::ChangePasswordReqArgs,
    uac::{ChangePasswordError, PasswordComplexity, UserInfo},
};

#[tracing::instrument(skip(req_args, pool))]
pub async fn change_password(
    req_args: web::Json<ChangePasswordReqArgs>,
    pool: web::Data<DbPool>,
    login_attempt_limit: web::Data<LoginAttemptLimit>,
    user_info: web::ReqData<UserInfo>,
) -> Result<HttpResponse, ChangePasswordError> {
    let username = user_info.into_inner().username;
    let password_complexity = PasswordComplexity::new(&username, &req_args.new_password);
    if !password_complexity.does_meet_requirements() {
        return Err(ChangePasswordError::Complexity(password_complexity));
    }
    if req_args.new_password.expose_secret() != req_args.new_password_check.expose_secret() {
        return Err(ChangePasswordError::PasswordsDoNotMatch);
    }

    let credentials = Credentials {
        username: username.clone().into(),
        password: req_args.0.current_password,
    };

    validate_credentials(credentials, &pool, &login_attempt_limit).await?;

    let should_force_pass_change = false;
    crate::authentication::change_password(
        &username,
        req_args.0.new_password,
        should_force_pass_change,
        &pool,
    )
    .await
    .map_err(ChangePasswordError::UnexpectedError)?;

    Ok(HttpResponse::Ok().finish())
}
