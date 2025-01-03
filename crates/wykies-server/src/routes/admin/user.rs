#[cfg(feature = "mysql")]
use crate::db_utils::db_int_to_bool;
use crate::{authentication, db_utils::validate_one_row_affected};
use actix_web::{web, HttpResponse};
use anyhow::Context;
use argon2::password_hash::SaltString;
use argon2::PasswordHasher;
use secrecy::ExposeSecret;
use wykies_shared::db_types::DbPool;
use wykies_shared::{
    e400, e500,
    req_args::{
        api::admin::user::{self, NewUserReqArgs, PasswordResetReqArgs},
        RonWrapper,
    },
    session::UserSessionInfo,
    uac::{ListUsersRoles, ResetPasswordError, RoleIdAndName, UserMetadata, UserMetadataDiff},
};

#[tracing::instrument(ret, err(Debug), skip(pool))]
pub async fn user(
    pool: web::Data<DbPool>,
    web::Query(user::LookupReqArgs { username }): web::Query<user::LookupReqArgs>,
) -> actix_web::Result<web::Json<UserMetadata>> {
    let pool: &DbPool = &pool;
    #[cfg(feature = "mysql")]
    let query = sqlx::query!(
        "SELECT `UserName`, `DisplayName`, `ForcePassChange`, `AssignedRole`, `Enabled`, `LockedOut`, `FailedAttempts`, `PassChangeDate`
         FROM `user`
         WHERE UserName=?;",
    username);
    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    // TODO 5: Check why encode trait impl doesn't make converting not necessary
    let query = sqlx::query!(
        "SELECT user_name, display_name, force_pass_change, assigned_role, is_enabled, locked_out, failed_attempts, pass_change_date
         FROM users
         WHERE user_name=$1;",
        username.as_ref()
    );
    let Some(record) = query
        .fetch_optional(pool)
        .await
        .context("failed to get user")
        .map_err(e500)?
    else {
        return Err(e400("no user found with that username"));
    };

    #[cfg(feature = "mysql")]
    let result = UserMetadata {
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
        failed_attempts: record.FailedAttempts.try_into().map_err(e500)?,
        pass_change_date: record.PassChangeDate,
    };
    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    let result = UserMetadata {
        username: record.user_name.try_into().map_err(e500)?,
        display_name: record.display_name.try_into().map_err(e500)?,
        force_pass_change: record.force_pass_change,
        assigned_role: if let Some(x) = record.assigned_role {
            Some(x.try_into().map_err(e500)?)
        } else {
            None
        },
        enabled: record.is_enabled,
        locked_out: record.locked_out,
        failed_attempts: record.failed_attempts.try_into().map_err(e500)?,
        pass_change_date: record.pass_change_date,
    };

    Ok(web::Json(result))
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
    #[cfg(feature = "mysql")]
    let query  = sqlx::query!(
        "INSERT INTO `user`
        (`UserName`, `Password`, `password_hash`, `salt`, `DisplayName`, `AssignedRole`, `PassChangeDate`, `Enabled`) 
        VALUES (?, '', ?, '', ?, ?, CURRENT_DATE(), 1);",
        args.username,
        password_hash,
        args.display_name,
        args.assigned_role
    );
    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    // TODO 5: Check why encode trait impl doesn't make converting not necessary
    let query = {
        let assigned_role: Option<i32> = match args.assigned_role {
            Some(x) => Some(x.try_into().map_err(e500)?),
            None => None,
        };
        sqlx::query!(
            "INSERT INTO users
        (user_name, password_hash, display_name, assigned_role, pass_change_date, is_enabled) 
        VALUES ($1, $2, $3, $4, CURRENT_DATE, true);",
            args.username.as_ref(),
            password_hash,
            args.display_name.as_ref(),
            assigned_role
        )
    };
    let sql_result = query
        .execute(pool)
        .await
        .context("failed to store user")
        .map_err(e500)?;
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
    // TODO 2: Ensure there is a test that assigns a role, changes a role and
    //          removes a role
    let pool: &DbPool = &pool;
    let diff: UserMetadataDiff = wrapped
        .deserialize()
        .context("convert from ron failed")
        .map_err(e400)?;
    diff.is_valid().map_err(e400)?;
    #[cfg(feature = "mysql")]
    let query = sqlx::query!(
        "UPDATE `user` SET
        `DisplayName` = CASE WHEN ? IS NULL THEN `DisplayName` ELSE ? end,
        `ForcePassChange` = CASE WHEN ? IS NULL THEN `ForcePassChange` ELSE ? end,
        `AssignedRole` = CASE WHEN ? <> 0 THEN `AssignedRole` ELSE ? end,
        `Enabled` = CASE WHEN ? IS NULL THEN `Enabled` ELSE ? end,
        `LockedOut` = CASE WHEN ? IS NULL THEN `LockedOut` ELSE ? end,
        `FailedAttempts` = CASE WHEN ? IS NULL THEN `FailedAttempts` ELSE ? end
        WHERE `UserName`=?;",
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
    );
    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    // TODO 5: Check why encode trait impl doesn't make converting not necessary
    let query = {
        let display_name = diff.display_name.map(|x| x.to_string());
        let assigned_role: Option<i32> = match diff.assigned_role {
            Some(Some(x)) => Some(x.try_into().map_err(e500)?),
            Some(None) | None => None,
        };
        let failed_attempts: Option<i16> = diff.failed_attempts.map(|x| x.into());
        sqlx::query!(
            "UPDATE users SET
            display_name = CASE WHEN $1 THEN display_name ELSE $2 end,
            force_pass_change = CASE WHEN $3 THEN force_pass_change ELSE $4 end,
            assigned_role = CASE WHEN $5 THEN assigned_role ELSE $6 end,
            is_enabled = CASE WHEN $7 THEN is_enabled ELSE $8 end,
            locked_out = CASE WHEN $9 THEN locked_out ELSE $10 end,
            failed_attempts = CASE WHEN $11 THEN failed_attempts ELSE $12 end
            WHERE user_name=$13",
            display_name.is_none(),
            display_name,
            diff.force_pass_change.is_none(),
            diff.force_pass_change,
            diff.assigned_role.is_none(),
            assigned_role,
            diff.enabled.is_none(),
            diff.enabled,
            diff.locked_out.is_none(),
            diff.locked_out,
            diff.failed_attempts.is_none(),
            failed_attempts,
            diff.username.as_ref()
        )
    };
    let sql_result = query
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
    #[cfg(feature = "mysql")]
    let query = sqlx::query!("SELECT `RoleID`, `Name` FROM `roles`");
    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    let query = sqlx::query!("SELECT role_id, role_name FROM roles");
    query
        .fetch_all(pool)
        .await
        .context("failed to get list of roles")
        .map_err(e500)?
        .into_iter()
        .map(|x| {
            #[cfg(feature = "mysql")]
            return Ok(RoleIdAndName {
                id: x.RoleID.try_into()?,
                name: x.Name.try_into()?,
            });
            #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
            Ok(RoleIdAndName {
                id: x.role_id.try_into()?,
                name: x.role_name.try_into()?,
            })
        })
        .collect::<anyhow::Result<Vec<RoleIdAndName>>>()
        .map_err(e500)
}

async fn get_users_list(pool: &DbPool) -> actix_web::Result<Vec<UserMetadata>> {
    #[cfg(feature = "mysql")]
    let query = sqlx::query!("SELECT `UserName`, `DisplayName`, `ForcePassChange`, `AssignedRole`, `Enabled`, `LockedOut`, `FailedAttempts`, `PassChangeDate` FROM `user`",);
    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    let query = sqlx::query!("SELECT user_name, display_name, force_pass_change, assigned_role, is_enabled, locked_out, failed_attempts, pass_change_date FROM users",);
    query
        .fetch_all(pool)
        .await
        .context("failed to get list of users")
        .map_err(e500)?
        .into_iter()
        .map(|x| {
            #[cfg(feature = "mysql")]
            return Ok(UserMetadata {
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
                failed_attempts: x.FailedAttempts.try_into()?,
                pass_change_date: x.PassChangeDate,
            });
            #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
            Ok(UserMetadata {
                username: x.user_name.try_into()?,
                display_name: x.display_name.try_into()?,
                force_pass_change: x.force_pass_change,
                assigned_role: if let Some(x) = x.assigned_role {
                    Some(x.try_into()?)
                } else {
                    None
                },
                enabled: x.is_enabled,
                locked_out: x.locked_out,
                failed_attempts: x.failed_attempts.try_into()?,
                pass_change_date: x.pass_change_date,
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
