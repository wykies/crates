use crate::db_utils::validate_one_row_affected;
use actix_web::{web, HttpResponse};
use anyhow::Context;
use wykies_shared::db_types::DbPool;
use wykies_shared::{
    e400, e500,
    id::DbId,
    req_args::api::admin::role::{self, AssignReqArgs},
    uac::{Role, RoleDraft},
};

#[tracing::instrument(skip(pool))]
pub async fn role(
    pool: web::Data<DbPool>,
    web::Query(role::LookupReqArgs { role_id }): web::Query<role::LookupReqArgs>,
) -> actix_web::Result<web::Json<Role>> {
    let pool: &DbPool = &pool;
    let row = sqlx::query!(
        "SELECT `RoleID`, `Name`, `Description`, `Permissions` FROM `roles` WHERE `RoleID` = ?",
        role_id
    )
    .fetch_one(pool)
    .await
    .context("failed to find role")
    .map_err(e400)?;
    let result = Role {
        id: role_id,
        name: row.Name.try_into().map_err(e500)?,
        description: row.Description.try_into().map_err(e500)?,
        permissions: row.Permissions.try_into().map_err(e500)?,
    };
    Ok(web::Json(result))
}

#[tracing::instrument(skip(pool))]
pub async fn role_create(
    pool: web::Data<DbPool>,
    web::Json(draft_role): web::Json<RoleDraft>,
) -> actix_web::Result<web::Json<DbId>> {
    let pool: &DbPool = &pool;
    let permissions: String = draft_role.permissions.into();
    let sql_result = sqlx::query!(
        "INSERT INTO `roles` 
    (`RoleID`, `Name`, `Description`, `Permissions`, `LockedEditing`)
    VALUES (NULL, ?, ?, ?, '0');",
        draft_role.name,
        draft_role.description,
        permissions
    )
    .execute(pool)
    .await
    .context("failed to insert role")
    .map_err(e500)?;
    validate_one_row_affected(&sql_result).map_err(e500)?;
    let result = sql_result.last_insert_id().into();
    Ok(web::Json(result))
}

#[tracing::instrument(skip(pool))]
pub async fn role_assign(
    pool: web::Data<DbPool>,
    web::Json(req_args): web::Json<AssignReqArgs>,
) -> actix_web::Result<HttpResponse> {
    let pool: &DbPool = &pool;
    let sql_result = sqlx::query!(
        "UPDATE `user`
        SET `AssignedRole` = ? 
        WHERE `user`.`UserName` = ?;",
        req_args.role_id,
        req_args.username
    )
    .execute(pool)
    .await
    .context("failed to set role for user")
    .map_err(e500)?;
    validate_one_row_affected(&sql_result).map_err(e500)?;
    Ok(HttpResponse::Ok().finish())
}
