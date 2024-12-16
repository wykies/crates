use actix_web::{web, HttpResponse};
use anyhow::Context as _;
use wykies_shared::db_types::DbPool;
use wykies_shared::{
    e500, host_branch::HostBranchPair, id::DbId, req_args::api::admin::host_branch,
};

#[tracing::instrument(ret, err(Debug), skip(pool))]
pub async fn set_host_branch_pair(
    pool: web::Data<DbPool>,
    web::Json(pair): web::Json<HostBranchPair>,
) -> actix_web::Result<HttpResponse> {
    let pool: &DbPool = &pool;
    sqlx::query!(
        "INSERT INTO `hostbranch` 
        (`hostname`, `AssignedBranch`)
        VALUES (?, ?) 
        ON DUPLICATE KEY UPDATE `AssignedBranch` = ?;",
        pair.host_id,
        pair.branch_id,
        pair.branch_id,
    )
    .execute(pool)
    .await
    .context("failed to set host_branch_pair")
    .map_err(e500)?;
    // Can not validate number of rows because it can change if update to same, insert new or update https://dev.mysql.com/doc/refman/8.4/en/insert-on-duplicate.html
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(ret, err(Debug), skip(pool))]
pub async fn list_host_branch_pairs(
    pool: web::Data<DbPool>,
) -> actix_web::Result<web::Json<Vec<HostBranchPair>>> {
    let pool: &DbPool = &pool;
    let rows = sqlx::query!("SELECT `hostname`, `AssignedBranch` FROM `hostbranch`",)
        .fetch_all(pool)
        .await
        .context("failed to get list of host_branch_pairs")
        .map_err(e500)?;
    let result = rows
        .into_iter()
        .map(|x| {
            Ok(HostBranchPair {
                host_id: x.hostname.try_into()?,
                branch_id: x.AssignedBranch.try_into()?,
            })
        })
        .collect::<anyhow::Result<Vec<HostBranchPair>>>()
        .map_err(e500)?;
    Ok(web::Json(result))
}

#[tracing::instrument(ret, err(Debug), skip(pool))]
pub async fn host_branch_pair_lookup(
    pool: web::Data<DbPool>,
    web::Query(host_branch::LookupReqArgs { host_id }): web::Query<host_branch::LookupReqArgs>,
) -> actix_web::Result<web::Json<Option<DbId>>> {
    let pool: &DbPool = &pool;
    let row = sqlx::query!(
        "SELECT `hostname`, `AssignedBranch` FROM `hostbranch` where `hostname` = ?",
        host_id
    )
    .fetch_optional(pool)
    .await
    .context("failed to lookup host_branch_pair")
    .map_err(e500)?;
    let Some(row) = row else {
        return Ok(web::Json(None));
    };
    let result = row.AssignedBranch.try_into().map_err(e500)?;
    Ok(web::Json(Some(result)))
}
