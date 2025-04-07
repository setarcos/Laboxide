use actix_web::{get, post, put, delete, web, HttpResponse, Responder};
use serde_json::json;
use sqlx::SqlitePool;

use crate::db;
use crate::models::Course;

#[post("/course")]
pub async fn create_course(
    db_pool: web::Data<SqlitePool>,
    item: web::Json<Course>,
) -> impl Responder {
    match db::add_course(&db_pool, item.into_inner()).await {
        Ok(course) => HttpResponse::Ok().json(course),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/course")]
pub async fn list_courses(
    db_pool: web::Data<SqlitePool>,
) -> impl Responder {
    match db::list_courses(&db_pool).await {
        Ok(courses) => HttpResponse::Ok().json(courses),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/course/{id}")]
pub async fn get_course(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let id = path.into_inner();
    match db::get_course_by_id(&db_pool, id).await {
        Ok(Some(course)) => HttpResponse::Ok().json(course),
        Ok(None) => HttpResponse::NotFound().json(json!({ "error": "Course not found" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[put("/course/{id}")]
pub async fn update_course(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    item: web::Json<Course>,
) -> impl Responder {
    let id = path.into_inner();
    match db::update_course(&db_pool, id, item.into_inner()).await {
        Ok(Some(course)) => HttpResponse::Ok().json(course),
        Ok(None) => HttpResponse::NotFound().json(json!({ "error": "Course not found" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[delete("/course/{id}")]
pub async fn delete_course(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let id = path.into_inner();
    match db::delete_course(&db_pool, id).await {
        Ok(true) => HttpResponse::Ok().json(json!({ "message": "Course deleted" })),
        Ok(false) => HttpResponse::NotFound().json(json!({ "error": "Course not found" })),
        Err(_) => HttpResponse::InternalServerError().json(json!({ "error": "Failed to delete course" })),
    }
}

pub fn init_course_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_course)
        .service(get_course)
        .service(list_courses)
        .service(update_course)
        .service(delete_course);
}
