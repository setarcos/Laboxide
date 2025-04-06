use actix_web::{get, post, put, delete, web, HttpResponse, Responder};
use sqlx::SqlitePool;

use crate::db;
use crate::models::Semester;

#[post("/semester")]
pub async fn create_semester(
    db_pool: web::Data<SqlitePool>,
    item: web::Json<Semester>,
) -> impl Responder {
    match db::create_semester(&db_pool, item.into_inner()).await {
        Ok(semester) => HttpResponse::Ok().json(semester),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() })),
    }
}

#[get("/semester")]
pub async fn list_semesters(
    db_pool: web::Data<SqlitePool>,
) -> impl Responder {
    match db::get_all_semesters(&db_pool).await {
        Ok(semesters) => HttpResponse::Ok().json(semesters),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() })),
    }
}

#[get("/semester/{id}")]
pub async fn get_semester_by_id(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let id = path.into_inner();
    match db::get_semester_by_id(&db_pool, id).await {
        Ok(Some(semester)) => HttpResponse::Ok().json(semester),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({ "error": "Semester not found" })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() })),
    }
}

#[put("/semester/{id}")]
pub async fn update_semester(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    item: web::Json<Semester>,
) -> impl Responder {
    let id = path.into_inner();
    match db::update_semester(&db_pool, id, item.into_inner()).await {
        Ok(Some(semester)) => HttpResponse::Ok().json(semester),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({ "error": "Semester not found" })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e.to_string() })),
    }
}

#[delete("/semester/{id}")]
pub async fn delete_semester(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let id = path.into_inner();
    match db::delete_semester(&db_pool, id).await {
        Ok(true) => HttpResponse::Ok().json(serde_json::json!({ "message": "Semester deleted" })),
        Ok(false) => HttpResponse::NotFound().json(serde_json::json!({ "error": "Semester not found" })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": "Failed to delete semester" })),
    }
}

pub fn init_semester_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_semester)
        .service(get_semester_by_id)
        .service(list_semesters)
        .service(update_semester)
        .service(delete_semester);
}
