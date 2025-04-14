use actix_web::{post, put, web, get, HttpResponse, Responder};
use serde_json::json;
use sqlx::SqlitePool;
use serde::Deserialize;
use crate::db;
use crate::models::StudentLog;

#[post("/student_log")]
pub async fn create_student_log(
    db_pool: web::Data<SqlitePool>,
    item: web::Json<StudentLog>,
) -> impl Responder {
    match db::add_student_log(&db_pool, item.into_inner()).await {
        Ok(log) => HttpResponse::Ok().json(log),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[put("/student_log/{id}")]
pub async fn update_student_log(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    item: web::Json<StudentLog>,
) -> impl Responder {
    let id = path.into_inner();
    match db::update_student_log(&db_pool, id, item.into_inner()).await {
        Ok(()) => HttpResponse::Ok().json(json!({"status": "updated"})),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[derive(Debug, Deserialize)]
pub struct TeacherConfirmRequest {
    pub tea_note: String,
}

#[put("/student_log/{id}/confirm")]
pub async fn confirm_student_log(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    item: web::Json<TeacherConfirmRequest>,
) -> impl Responder {
    let id = path.into_inner();
    match db::confirm_student_log(&db_pool, id, &item.into_inner().tea_note).await {
        Ok(_) => HttpResponse::Ok().json(json!({ "status": "confirmed" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[derive(Deserialize)]
pub struct GetLogParams {
    pub subcourse_id: i64,
    pub stu_id: String,
}

#[get("/student_log/default")]
async fn default_student_log(
    pool: web::Data<SqlitePool>,
    query: web::Query<GetLogParams>,
) -> impl Responder {
    match db::get_default_log(&pool, &query.stu_id, query.subcourse_id).await {
        Ok(log) => HttpResponse::Ok().json(log),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

pub fn init_student_log_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_student_log)
       .service(update_student_log);
}
