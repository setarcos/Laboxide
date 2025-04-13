use actix_session::Session;
use actix_web::{get, put, post, delete, web, HttpResponse, Responder};
use sqlx::SqlitePool;
use serde::Deserialize;
use serde_json::json;
use crate::db;
use log::error;

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

#[delete("/groupremove/{subcourse_id}/{stu_id}")]
pub async fn remove_student(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<(i64, String)>,
) -> impl Responder {
    let (subcourse_id, stu_id) = path.into_inner();

    match db::remove_student_from_group(&db_pool, &stu_id, subcourse_id).await {
        Ok(_) => HttpResponse::Ok().json(json!({ "status": "student removed" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[derive(Debug, Deserialize)]
pub struct SeatUpdateRequest {
    pub stu_id: String,
    pub subcourse_id: i64,
    pub seat: i64,
}

#[put("/group/seat")]
pub async fn update_student_seat(
    db_pool: web::Data<SqlitePool>,
    payload: web::Json<SeatUpdateRequest>,
) -> impl Responder {
    let SeatUpdateRequest {
        stu_id,
        subcourse_id,
        seat,
    } = payload.into_inner();

    match db::set_student_seat(&db_pool, &stu_id, subcourse_id, seat).await {
        Ok(_) => HttpResponse::Ok().json(json!({ "message": "Seat updated successfully" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

// Register the routes
pub fn init_group_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(join_group)
       .service(leave_group)
       .service(list_group);
}
