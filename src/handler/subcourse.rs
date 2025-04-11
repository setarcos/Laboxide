use actix_web::{get, post, put, delete, web, HttpResponse, Responder};
use serde_json::json;
use sqlx::SqlitePool;
use serde::Deserialize;

use crate::db;
use crate::models::SubCourse;

#[post("/subcourse")]
pub async fn create_subcourse(
    db_pool: web::Data<SqlitePool>,
    item: web::Json<SubCourse>,
) -> impl Responder {
    match db::add_subcourse(&db_pool, item.into_inner()).await {
        Ok(subcourse) => HttpResponse::Ok().json(subcourse),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[derive(Deserialize)]
pub struct SubcourseQuery {
    pub course_id: Option<i64>,
    pub semester_id: Option<i64>,
}

#[get("/subcourse")]
pub async fn list_subcourses(
    db_pool: web::Data<sqlx::SqlitePool>,
    query: web::Query<SubcourseQuery>,
) -> impl Responder {
    let course_id = query.course_id;
    let semester_id = query.semester_id;

    match db::list_subcourses(&db_pool, course_id, semester_id).await {
        Ok(subcourses) => HttpResponse::Ok().json(subcourses),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/subcourse/{id}")]
pub async fn get_subcourse(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    match db::get_subcourse_by_id(&db_pool, path.into_inner()).await {
        Ok(Some(subcourse)) => HttpResponse::Ok().json(subcourse),
        Ok(None) => HttpResponse::NotFound().json(json!({ "error": "SubCourse not found" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[put("/subcourse/{id}")]
pub async fn update_subcourse(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    item: web::Json<SubCourse>,
) -> impl Responder {
    let id = path.into_inner();
    match db::update_subcourse(&db_pool, id, item.into_inner()).await {
        Ok(Some(subcourse)) => HttpResponse::Ok().json(subcourse),
        Ok(None) => HttpResponse::NotFound().json(json!({ "error": "SubCourse not found" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[delete("/subcourse/{id}")]
pub async fn delete_subcourse(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    match db::delete_subcourse(&db_pool, path.into_inner()).await {
        Ok(true) => HttpResponse::Ok().json(json!({ "message": "SubCourse deleted" })),
        Ok(false) => HttpResponse::NotFound().json(json!({ "error": "SubCourse not found" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

pub fn init_subcourse_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_subcourse)
        .service(list_subcourses)
        .service(get_subcourse)
        .service(update_subcourse)
        .service(delete_subcourse);
}
