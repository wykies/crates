use crate::db_types::DbPool;
use actix_web::{web, HttpResponse};

pub async fn health_check(_pool: web::Data<DbPool>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
