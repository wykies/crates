use actix_web::{web, HttpResponse};
use wykies_shared::db_types::DbPool;

pub async fn health_check(_pool: web::Data<DbPool>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
