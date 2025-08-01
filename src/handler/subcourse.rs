use actix_web::{get, post, put, delete, web, HttpResponse, Responder};
use actix_session::Session;
use serde_json::json;
use sqlx::SqlitePool;
use serde::Deserialize;
use crate::config::{PERMISSION_STUDENT, PERMISSION_TEACHER, PERMISSION_LINUX};
use log::error;
use crate::utils::check_course_perm;

use crate::db;
use crate::models::SubCourse;

#[post("/subcourse")]
pub async fn create_subcourse(
    db_pool: web::Data<SqlitePool>,
    item: web::Json<SubCourse>,
    session: Session,
) -> impl Responder {
    let sub = item.into_inner();

    if let Err(err) = check_course_perm(&db_pool, &session, sub.course_id).await {
        return err;
    }
    match db::add_subcourse(&db_pool, sub).await {
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
    session: Session,
) -> impl Responder {
    let course_id = query.course_id;
    let semester_id = query.semester_id;
    let permission: i64 = session.get::<i64>("permissions").ok().flatten().unwrap_or(0);

    match db::list_subcourses(&db_pool, course_id, semester_id).await {
        Ok(mut subcourses) => {
            if permission & PERMISSION_TEACHER == 0 {
                for subcourse in &mut subcourses{
                    subcourse.tea_id = String::new();
                }
            }
            HttpResponse::Ok().json(subcourses)
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/mycourse")]
pub async fn list_my_subcourses(
    db_pool: web::Data<sqlx::SqlitePool>,
    session: Session,
) -> impl Responder {

    let mut permission: i64 = session.get::<i64>("permissions").ok().flatten().unwrap_or(0);
    let user_id: String = session.get::<String>("user_id").ok().flatten().unwrap_or("".to_string());

    if user_id.is_empty() {
        return HttpResponse::Unauthorized().json(json!({ "error": "User not logged in" }));
    }

    let result = if (permission & PERMISSION_TEACHER) != 0 {
        db::list_teacher_subcourses(&db_pool, &user_id).await
    } else if (permission & PERMISSION_STUDENT) != 0 {
        db::list_student_subcourses(&db_pool, &user_id).await
    } else {
        Err(sqlx::Error::RowNotFound) // Or handle unknown permission better
    };

    match result {
        Ok(mut subcourses) =>{
            if permission & PERMISSION_TEACHER == 0 {
                for subcourse in &mut subcourses{
                    if subcourse.course_name.starts_with("Linux") {
                        permission |= PERMISSION_LINUX;
                        let _ = session.insert("permissions", permission);
                    }
                    subcourse.tea_id = String::new();
                }
            }
            HttpResponse::Ok().json(subcourses)
        }
        Err(e) => {
            error!("Error listing subcourses: {:?}", e);
            HttpResponse::InternalServerError().json(json!({ "error": "Failed to fetch subcourses" }))
        }
    }
}

#[get("/subcourse/{id}")]
pub async fn get_subcourse(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    session: Session,
) -> impl Responder {
    let permission: i64 = session.get::<i64>("permissions").ok().flatten().unwrap_or(0);
    match db::get_subcourse_with_name(&db_pool, path.into_inner()).await {
        Ok(mut subcourse) => {
            if permission & PERMISSION_TEACHER == 0 {
                subcourse.tea_id = String::new();
            }
            HttpResponse::Ok().json(subcourse)
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[put("/subcourse/{id}")]
pub async fn update_subcourse(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    item: web::Json<SubCourse>,
    session: Session,
) -> impl Responder {
    let id = path.into_inner();
    let sub = match ensure_subcourse_exists(&db_pool, id).await {
        Ok(s) => s,
        Err(resp) => return resp,
    };
    if let Err(err) = check_course_perm(&db_pool, &session, sub.course_id).await {
        return err;
    }
    match db::update_subcourse(&db_pool, id, item.into_inner()).await {
        Ok(subcourse) => HttpResponse::Ok().json(subcourse),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

pub async fn ensure_subcourse_exists(
    db_pool: &SqlitePool,
    subcourse_id: i64,
) -> Result<SubCourse, HttpResponse> {
    match crate::db::get_subcourse_by_id(db_pool, subcourse_id).await {
        Ok(sub) => Ok(sub),
        Err(e) => Err(HttpResponse::InternalServerError().json(json!({ "error": e.to_string() }))),
    }
}

#[delete("/subcourse/{id}")]
pub async fn delete_subcourse(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    session: Session,
) -> impl Responder {
    let id = path.into_inner();
    let sub = match ensure_subcourse_exists(&db_pool, id).await {
        Ok(s) => s,
        Err(resp) => return resp,
    };
    if let Err(err) = check_course_perm(&db_pool, &session, sub.course_id).await {
        return err;
    }
    match db::delete_subcourse(&db_pool, id).await {
        Ok(true) => HttpResponse::Ok().json(json!({ "message": "SubCourse deleted" })),
        Ok(false) => HttpResponse::NotFound().json(json!({ "error": "SubCourse not found" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

pub fn init_subcourse_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_subcourse)
        .service(update_subcourse)
        .service(delete_subcourse);
}
