use actix_session::Session;
use actix_web::{get, put, post, delete, web, HttpResponse, Responder};
use sqlx::SqlitePool;
use serde_json::json;
use crate::db;
use log::error;
use crate::utils::check_subcourse_perm;

// Add current user to group
#[post("/group/join/{subcourse_id}")]
pub async fn join_group(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    session: Session,
) -> impl Responder {
    let subcourse_id = path.into_inner();
    let user_id_res = session.get::<String>("user_id");
    let realname_res = session.get::<String>("realname");

    match (user_id_res, realname_res) {
        (Ok(Some(user_id)), Ok(Some(realname))) => {
            match db::add_student_to_group(&db_pool, &user_id, &realname, subcourse_id).await {
                Ok(_) => HttpResponse::Ok().json(json!({ "status": "added" })),
                Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
            }
        }

        (Err(e), _) | (_, Err(e)) => {
            error!("Session get error: {:?}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to retrieve session data"
            }))
        }

        // --- Unauthorized Case ---
        _ => {
            HttpResponse::Unauthorized().json(json!({
                "error": "Not logged in or session data incomplete"
            }))
        }
    }
}

// Remove current user from group
#[delete("/group/leave/{subcourse_id}")]
pub async fn leave_group(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    session: Session,
) -> impl Responder {
    let subcourse_id = path.into_inner();

    if let Ok(Some(user_id)) = session.get::<String>("user_id") {
        match db::remove_student_from_group(&db_pool, &user_id, subcourse_id).await {
            Ok(_) => HttpResponse::Ok().json(json!({ "status": "left" })),
            Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
        }
    } else {
        HttpResponse::Unauthorized().json(json!({
            "error": "Not logged in or session data incomplete"
        }))
    }
}

// List all students in the group for given subcourse_id
#[get("/group/{subcourse_id}")]
pub async fn list_group(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let subcourse_id = path.into_inner();

    match db::get_group_by_subcourse_id(&db_pool, subcourse_id).await {
        Ok(group) => HttpResponse::Ok().json(group),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[delete("/group/remove/{subcourse_id}/{stu_id}")]
pub async fn remove_student(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<(i64, String)>,
    session: Session,
) -> impl Responder {
    let (subcourse_id, stu_id) = path.into_inner();

    if let Err(err) = check_subcourse_perm(&db_pool, &session, subcourse_id).await {
        return err;
    }
    match db::remove_student_from_group(&db_pool, &stu_id, subcourse_id).await {
        Ok(_) => HttpResponse::Ok().json(json!({ "status": "student removed" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[put("/group/seat/{group_id}/{seat}")]
pub async fn update_student_seat(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<(i64, i64)>,
    session: Session,
) -> impl Responder {
    let (group_id, seat) = path.into_inner();

    if let Ok(Some(student)) = db::get_student_by_group_id(&db_pool, group_id).await {
        if let Err(err) = check_subcourse_perm(&db_pool, &session, student.subcourse_id).await {
            return err;
        }
        match db::set_student_seat(&db_pool, group_id, seat).await {
            Ok(_) => HttpResponse::Ok().json(json!({ "message": "Seat updated successfully" })),
            Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
        }
    } else {
        HttpResponse::NotFound().json(json!({ "error": "Student not found" }))
    }
}

// Register the routes
pub fn init_group_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(join_group)
       .service(leave_group)
       .service(list_group);
}
