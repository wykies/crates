#[cfg(feature = "mysql")]
use crate::db_utils::validate_one_row_affected;
use actix_web::web;
use anyhow::Context;
use wykies_shared::db_types::DbPool;
use wykies_shared::{
    branch::{Branch, BranchDraft},
    e500,
    id::DbId,
};

#[tracing::instrument(skip(pool))]
pub async fn branch_list(pool: web::Data<DbPool>) -> actix_web::Result<web::Json<Vec<Branch>>> {
    let pool: &DbPool = &pool;
    #[cfg(feature = "mysql")]
    let query = sqlx::query!("SELECT `BranchID`, `BranchName` FROM `branch`");
    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    let query = sqlx::query!("SELECT branch_id, branch_name FROM branch");
    let rows = query
        .fetch_all(pool)
        .await
        .context("failed to get branches")
        .map_err(e500)?;
    let result = rows
        .into_iter()
        .map(|x| {
            #[cfg(feature = "mysql")]
            return Ok(Branch {
                id: x.BranchID.try_into()?,
                name: x.BranchName.try_into().context("invalid branch name")?,
            });
            #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
            Ok(Branch {
                id: x.branch_id.try_into()?,
                name: x.branch_name.try_into().context("invalid branch name")?,
            })
        })
        .collect::<anyhow::Result<Vec<Branch>>>()
        .map_err(e500)?;
    Ok(web::Json(result))
}

#[tracing::instrument(ret, skip(pool))]
pub async fn branch_create(
    pool: web::Data<DbPool>,
    web::Json(draft): web::Json<BranchDraft>,
) -> actix_web::Result<web::Json<DbId>> {
    let pool: &DbPool = &pool;
    #[cfg(feature = "mysql")]
    let result = {
        let sql_result = sqlx::query!(
            "INSERT INTO `branch` 
            (`BranchID`, `BranchName`) 
            VALUES (NULL, ?);",
            draft.name,
        )
        .execute(pool)
        .await
        .context("failed to insert branch")
        .map_err(e500)?;
        validate_one_row_affected(&sql_result).map_err(e500)?;
        sql_result.last_insert_id().into()
    };
    #[cfg(all(not(feature = "mysql"), feature = "postgres"))]
    let result = {
        // TODO 5: Check why encode trait impl doesn't make converting not necessary
        sqlx::query!(
            "INSERT INTO branch
            (branch_name) 
            VALUES ($1) RETURNING branch_id;",
            draft.name.as_ref(),
        )
        .fetch_one(pool)
        .await
        .map_err(e500)?
        .branch_id
        .try_into()
        .map_err(e500)?
    };
    Ok(web::Json(result))
}
