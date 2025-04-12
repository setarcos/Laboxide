use actix_web::{get, post, put, delete, web, HttpResponse, Responder};
use serde_json::json;
use sqlx::SqlitePool;

use crate::db;
use crate::models::CourseSchedule;

#[post("/schedule")]
pub async fn create_schedule(
    db_pool: web::Data<SqlitePool>,
    item: web::Json<CourseSchedule>,
) -> impl Responder {
    match db::add_schedule(&db_pool, item.into_inner()).await {
        Ok(schedule) => HttpResponse::Ok().json(schedule),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/courseschedule/{id}")]
pub async fn list_schedules(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let id = path.into_inner();
    match db::list_schedules(&db_pool, id).await {
        Ok(schedules) => HttpResponse::Ok().json(schedules),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/schedule/{id}")]
pub async fn get_schedule(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let id = path.into_inner();
    match db::get_schedule_by_id(&db_pool, id).await {
        Ok(Some(schedule)) => HttpResponse::Ok().json(schedule),
        Ok(None) => HttpResponse::NotFound().json(json!({ "error": "CourseSchedule not found" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[put("/schedule/{id}")]
pub async fn update_schedule(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    item: web::Json<CourseSchedule>,
) -> impl Responder {
    let id = path.into_inner();
    match db::update_schedule(&db_pool, id, item.into_inner()).await {
        Ok(Some(schedule)) => HttpResponse::Ok().json(schedule),
        Ok(None) => HttpResponse::NotFound().json(json!({ "error": "CourseSchedule not found" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[delete("/schedule/{id}")]
pub async fn delete_schedule(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let id = path.into_inner();
    match db::delete_schedule(&db_pool, id).await {
        Ok(true) => HttpResponse::Ok().json(json!({ "message": "CourseSchedule deleted" })),
        Ok(false) => HttpResponse::NotFound().json(json!({ "error": "CourseSchedule not found" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

pub fn init_schedule_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_schedule)
        .service(get_schedule)
        .service(update_schedule)
        .service(delete_schedule);
}

