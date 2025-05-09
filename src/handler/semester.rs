use actix_web::{get, post, put, delete, web, HttpResponse, Responder};
use serde_json::json;
use sqlx::SqlitePool;

use crate::db;
use crate::models::Semester;

#[post("/semester")]
pub async fn create_semester(
    db_pool: web::Data<SqlitePool>,
    item: web::Json<Semester>,
) -> impl Responder {
    match db::add_semester(&db_pool, item.into_inner()).await {
        Ok(semester) => HttpResponse::Ok().json(semester),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/semester")]
pub async fn list_semesters(
    db_pool: web::Data<SqlitePool>,
) -> impl Responder {
    match db::list_semesters(&db_pool).await {
        Ok(semesters) => HttpResponse::Ok().json(semesters),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/semester/{id}")]
pub async fn get_semester(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let id = path.into_inner();
    match db::get_semester_by_id(&db_pool, id).await {
        Ok(semester) => HttpResponse::Ok().json(semester),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
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
        Ok(semester) => HttpResponse::Ok().json(semester),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[delete("/semester/{id}")]
pub async fn delete_semester(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let id = path.into_inner();
    match db::delete_semester(&db_pool, id).await {
        Ok(true) => HttpResponse::Ok().json(json!({ "message": "Semester deleted" })),
        Ok(false) => HttpResponse::NotFound().json(json!({ "error": "Semester not found" })),
        Err(_) => HttpResponse::InternalServerError().json(json!({ "error": "Failed to delete semester" })),
    }
}

#[get("/semester/current")]
pub async fn get_current_semester(
    db_pool: web::Data<SqlitePool>,
) -> impl Responder {
    match db::get_current_semester(&db_pool).await {
        Ok(Some(semester)) => HttpResponse::Ok().json(semester),
        Ok(None) => HttpResponse::Ok().json(json!({ "message": "Currently on vacation" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

pub fn init_semester_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_semester)
        .service(get_semester)
        .service(list_semesters)
        .service(update_semester)
        .service(delete_semester);
}
