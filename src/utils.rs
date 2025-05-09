use serde_json::json;
use sqlx::SqlitePool;
use actix_session::Session;
use crate::config::PERMISSION_ADMIN;
use actix_web::{HttpResponse, web};
use crate::db;

pub async fn check_course_perm(
    db_pool: &web::Data<SqlitePool>,
    session: &Session,
    course_id: i64,
) -> Result<(), HttpResponse> {
    let permission: i64 = session.get::<i64>("permissions").ok().flatten().unwrap_or(0);
    if permission & PERMISSION_ADMIN != 0 {
        return Ok(())
    }
    match db::get_course_by_id(db_pool, course_id).await {
        Ok(course) => {
            let user_id: String = session.get::<String>("user_id").ok().flatten().unwrap_or_default();
            if user_id != course.tea_id {
                return Err(HttpResponse::Forbidden().json(json!({"error": "Only course moderator can execute."})));
            }
            Ok(())
        }
        Err(e) => Err(HttpResponse::InternalServerError().json(json!({ "error": e.to_string() }))),
    }
}

pub async fn check_subcourse_perm(
    db_pool: &web::Data<SqlitePool>,
    session: &Session,
    subcourse_id: i64,
) -> Result<(), HttpResponse> {
    let permission: i64 = session.get::<i64>("permissions").ok().flatten().unwrap_or(0);
    if permission & PERMISSION_ADMIN != 0 {
        return Ok(())
    }
    match db::get_subcourse_by_id(db_pool, subcourse_id).await {
        Ok(subcourse) => {
            let user_id: String = session.get::<String>("user_id").ok().flatten().unwrap_or_default();
            if user_id != subcourse.tea_id {
                return Err(HttpResponse::Forbidden().json(json!({"error": "Only subcourse teacher can execute."})));
            }
            Ok(())
        }
        Err(e) => Err(HttpResponse::InternalServerError().json(json!({ "error": e.to_string() }))),
    }
}
