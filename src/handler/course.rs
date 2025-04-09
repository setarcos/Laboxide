use actix_web::{get, post, put, delete, web, HttpResponse, Responder};
use serde_json::json;
use sqlx::SqlitePool;
use actix_session::Session;

use crate::config::{PERMISSION_ADMIN, PERMISSION_TEACHER};
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
    session: Session,
) -> impl Responder {
    let id = path.into_inner();
    let permission: i64 = session.get::<i64>("permissions").ok().flatten().unwrap_or(0);
    let user_id: String = session.get::<String>("user_id").ok().flatten().unwrap_or("".to_string());

    match db::get_course_by_id(&db_pool, id).await {
        Ok(Some(course)) => {
            // Check if the user has permission to update the course
            if permission & PERMISSION_ADMIN != 0 || (permission & PERMISSION_TEACHER != 0 && course.tea_id == user_id) {
                // Proceed with update if authorized
                let mut newcourse = item.into_inner();
                if permission & PERMISSION_ADMIN == 0 {
                    // teacher can only change intro, tea_name and email
                    newcourse.tea_id = course.tea_id;
                    newcourse.name = course.name;
                    newcourse.ename = course.ename;
                    newcourse.code = course.code;
                    newcourse.term = course.term;
                }
                match db::update_course(&db_pool, id, newcourse).await {
                    Ok(Some(course)) => HttpResponse::Ok().json(course),
                    Ok(None) => HttpResponse::NotFound().json(json!({ "error": "Course not found" })),
                    Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
                }
            } else {
                // Deny access if the user is not authorized
                HttpResponse::Forbidden().json(json!({ "error": "Permission denied!" }))
            }
        }
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

pub fn init_course_adminroutes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_course)
        .service(update_course)
        .service(delete_course);
}
