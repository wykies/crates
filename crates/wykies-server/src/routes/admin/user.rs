use crate::{
    authentication,
    db_types::DbPool,
    db_utils::{db_int_to_bool, validate_one_row_affected},
};
use actix_web::{web, HttpResponse};
use anyhow::Context;
use argon2::password_hash::SaltString;
use argon2::PasswordHasher;
use secrecy::ExposeSecret;
use wykies_shared::{
    e400, e500,
    req_args::{
        api::admin::user::{self, NewUserReqArgs, PasswordResetReqArgs},
        RonWrapper,
    },
    session::UserSessionInfo,
    uac::{
        ListUsersRoles, ResetPasswordError, RoleIdAndName, UserMetadata, UserMetadataDiff, Username,
    },
};

#[tracing::instrument(ret, err(Debug), skip(pool))]
pub async fn user(
    pool: web::Data<DbPool>,
    web::Query(user::LookupReqArgs { username }): web::Query<user::LookupReqArgs>,
) -> actix_web::Result<web::Json<UserMetadata>> {
    let pool: &DbPool = &pool;
    let Some(record) = sqlx::query!(
        "SELECT `UserName`, `DisplayName`, `ForcePassChange`, `AssignedRole`, `Enabled`, `LockedOut`, `FailedAttempts`, `PassChangeDate`
         FROM `user`
         WHERE UserName=?",
    <Username as Into<String>>::into(username))
    .fetch_optional(pool)
    .await
    .context("failed to get user")
    .map_err(e500)?
    else {
        return Err(e400("no user found with that username"));
    };

    Ok(web::Json(UserMetadata {
        username: record.UserName.try_into().map_err(e500)?,
        display_name: record.DisplayName.try_into().map_err(e500)?,
        force_pass_change: db_int_to_bool(record.ForcePassChange),
        assigned_role: if let Some(x) = record.AssignedRole {
            Some(x.try_into().map_err(e500)?)
        } else {
            None
        },
        enabled: db_int_to_bool(record.Enabled),
        locked_out: db_int_to_bool(record.LockedOut),
        failed_attempts: record.FailedAttempts as _,
        pass_change_date: record.PassChangeDate,
    }))
}

#[tracing::instrument(ret, err(Debug), skip(pool))]
pub async fn user_new(
    pool: web::Data<DbPool>,
    web::Json(args): web::Json<NewUserReqArgs>,
) -> actix_web::Result<HttpResponse> {
    let pool: &DbPool = &pool;
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = authentication::argon2_settings()
        .hash_password(args.password.expose_secret().as_bytes(), &salt)
        .unwrap()
        .to_string();
    let sql_result = sqlx::query!(
        "INSERT INTO `user`
        (`UserName`, `Password`, `password_hash`, `salt`, `DisplayName`, `AssignedRole`, `PassChangeDate`, `Enabled`) 
        VALUES (?, '', ?, '', ?, ?, CURRENT_DATE(), 1);",
        args.username,
        password_hash,
        args.display_name,
        args.assigned_role
    )
    .execute(pool)
    .await
    .expect("failed to store test user");
    validate_one_row_affected(&sql_result)
        .context("failed to save new user")
        .map_err(e500)?;
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(ret, err(Debug), skip(pool))]
pub async fn user_update(
    pool: web::Data<DbPool>,
    wrapped: web::Json<RonWrapper>,
) -> actix_web::Result<actix_web::HttpResponse> {
    let pool: &DbPool = &pool;
    let diff: UserMetadataDiff = wrapped
        .deserialize()
        .context("convert from ron failed")
        .map_err(e400)?;
    diff.is_valid().map_err(e400)?;
    let sql_result = sqlx::query!(
        "UPDATE `user` SET
        `DisplayName` = CASE WHEN ? IS NULL THEN `DisplayName` ELSE ? end,
        `ForcePassChange` = CASE WHEN ? IS NULL THEN `ForcePassChange` ELSE ? end,
        `AssignedRole` = CASE WHEN ? <> 0 THEN `AssignedRole` ELSE ? end,
        `Enabled` = CASE WHEN ? IS NULL THEN `Enabled` ELSE ? end,
        `LockedOut` = CASE WHEN ? IS NULL THEN `LockedOut` ELSE ? end,
        `FailedAttempts` = CASE WHEN ? IS NULL THEN `FailedAttempts` ELSE ? end
        WHERE `UserName`=?",
        diff.display_name,
        diff.display_name,
        diff.force_pass_change,
        diff.force_pass_change,
        diff.assigned_role.is_none(),
        diff.assigned_role,
        diff.enabled,
        diff.enabled,
        diff.locked_out,
        diff.locked_out,
        diff.failed_attempts,
        diff.failed_attempts,
        diff.username
    )
    .execute(pool)
    .await
    .context("failed to update user")
    .map_err(e500)?;
    validate_one_row_affected(&sql_result)
        .context("wrong number of rows changed when updating user")
        .map_err(e500)?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(ret, err(Debug), skip(pool))]
pub async fn list_users_and_roles(
    pool: web::Data<DbPool>,
) -> actix_web::Result<web::Json<ListUsersRoles>> {
    let pool: &DbPool = &pool;
    let users = get_users_list(pool).await?;
    let roles = get_role_list(pool).await?;
    Ok(web::Json(ListUsersRoles { users, roles }))
}

async fn get_role_list(pool: &DbPool) -> actix_web::Result<Vec<RoleIdAndName>> {
    sqlx::query!("SELECT `RoleID`, `Name` FROM `roles`",)
        .fetch_all(pool)
        .await
        .context("failed to get list of roles")
        .map_err(e500)?
        .into_iter()
        .map(|x| {
            Ok(RoleIdAndName {
                id: x.RoleID.try_into()?,
                name: x.Name.try_into()?,
            })
        })
        .collect::<anyhow::Result<Vec<RoleIdAndName>>>()
        .map_err(e500)
}

async fn get_users_list(pool: &DbPool) -> actix_web::Result<Vec<UserMetadata>> {
    sqlx::query!("SELECT `UserName`, `DisplayName`, `ForcePassChange`, `AssignedRole`, `Enabled`, `LockedOut`, `FailedAttempts`, `PassChangeDate` FROM `user`",)
        .fetch_all(pool)
        .await
        .context("failed to get list of users")
        .map_err(e500)?
        .into_iter()
        .map(|x| {
            Ok(UserMetadata {
                username: x.UserName.try_into()?,
                display_name: x.DisplayName.try_into()?,
                force_pass_change: db_int_to_bool(x.ForcePassChange),
                assigned_role: if let Some(x) = x.AssignedRole {
                    Some(x.try_into()?)
                } else {
                    None
                },
                enabled: db_int_to_bool(x.Enabled),
                locked_out: db_int_to_bool(x.LockedOut),
                failed_attempts: x.FailedAttempts as _,
                pass_change_date: x.PassChangeDate
            })
        })
        .collect::<anyhow::Result<Vec<UserMetadata>>>()
        .map_err(e500)
}

#[tracing::instrument(skip(pool))]
pub async fn password_reset(
    pool: web::Data<DbPool>,
    web::Json(args): web::Json<PasswordResetReqArgs>,
    user_info: web::ReqData<UserSessionInfo>,
) -> Result<HttpResponse, ResetPasswordError> {
    let logged_in_username = user_info.into_inner().username;
    if logged_in_username == args.username {
        return Err(ResetPasswordError::NoResetOwnPassword);
    }

    let should_force_pass_change = true;
    crate::authentication::change_password(
        &args.username,
        args.new_password,
        should_force_pass_change,
        &pool,
    )
    .await
    .map_err(ResetPasswordError::UnexpectedError)?;

    Ok(HttpResponse::Ok().finish())
}
