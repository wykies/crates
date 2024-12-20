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

#[tracing::instrument(ret, err(Debug), skip(pool))]
pub async fn role(
    pool: web::Data<DbPool>,
    web::Query(role::LookupReqArgs { role_id }): web::Query<role::LookupReqArgs>,
) -> actix_web::Result<web::Json<Role>> {
    let pool: &DbPool = &pool;
    #[cfg(feature = "mysql")]
    let query = sqlx::query!(
        "SELECT `RoleID`, `Name`, `Description`, `Permissions` FROM `roles` WHERE `RoleID` = ?",
        role_id
    );
    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    let query = {
        let role_id: i32 = role_id.try_into().map_err(e500)?;
        sqlx::query!(
            "SELECT role_id, role_name, role_description, permissions FROM roles WHERE role_id = $1",
            role_id
        )
    };
    let row = query
        .fetch_one(pool)
        .await
        .context("failed to find role")
        .map_err(e400)?;
    #[cfg(feature = "mysql")]
    let result = Role {
        id: role_id,
        name: row.Name.try_into().map_err(e500)?,
        description: row.Description.try_into().map_err(e500)?,
        permissions: row.Permissions.try_into().map_err(e500)?,
    };
    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    let result = Role {
        id: role_id,
        name: row.role_name.try_into().map_err(e500)?,
        description: row.role_description.try_into().map_err(e500)?,
        permissions: row.permissions.try_into().map_err(e500)?,
    };
    Ok(web::Json(result))
}

#[tracing::instrument(ret, err(Debug), skip(pool))]
pub async fn role_create(
    pool: web::Data<DbPool>,
    web::Json(draft_role): web::Json<RoleDraft>,
) -> actix_web::Result<web::Json<DbId>> {
    let pool: &DbPool = &pool;
    let permissions: String = draft_role.permissions.into();
    #[cfg(feature = "mysql")]
    let result = {
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
        sql_result.last_insert_id().into()
    };

    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    // TODO 5: Check why encode trait impl doesn't make converting not necessary
    let result = {
        let name: &str = draft_role.name.as_ref();
        let description: &str = draft_role.description.as_ref();
        sqlx::query!(
            "INSERT INTO roles 
            (role_name, role_description, permissions)
            VALUES ($1, $2, $3) RETURNING role_id;",
            name,
            description,
            permissions
        )
        .fetch_one(pool)
        .await
        .map_err(e500)?
        .role_id
        .try_into()
        .map_err(e500)?
    };

    Ok(web::Json(result))
}

#[tracing::instrument(err(Debug), skip(pool))]
pub async fn role_assign(
    pool: web::Data<DbPool>,
    web::Json(req_args): web::Json<AssignReqArgs>,
) -> actix_web::Result<HttpResponse> {
    let pool: &DbPool = &pool;
    #[cfg(feature = "mysql")]
    let query = sqlx::query!(
        "UPDATE `user`
        SET `AssignedRole` = ? 
        WHERE `user`.`UserName` = ?;",
        req_args.role_id,
        req_args.username
    );
    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    // TODO 5: Check why encode trait impl doesn't make converting not necessary
    let query = {
        let role_id: i32 = req_args.role_id.try_into().map_err(e500)?;
        sqlx::query!(
            "UPDATE users
        SET assigned_role = $1 
        WHERE users.user_name = $2;",
            role_id,
            req_args.username.as_ref()
        )
    };
    let sql_result = query
        .execute(pool)
        .await
        .context("failed to set role for user")
        .map_err(e500)?;
    validate_one_row_affected(&sql_result).map_err(e500)?;
    Ok(HttpResponse::Ok().finish())
}
