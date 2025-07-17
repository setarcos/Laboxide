use actix_session::Session;
use actix_web::{get, post, put, delete, web, HttpResponse, Responder};
use serde_json::json;
use sqlx::SqlitePool;

use crate::db;
use crate::models::{MeetingRoom, MeetingAgenda};
use crate::config::{PERMISSION_MEETING_MANAGER, PERMISSION_ADMIN};

#[post("/meeting_room")]
pub async fn create_meeting_room(
    db_pool: web::Data<SqlitePool>,
    item: web::Json<MeetingRoom>,
) -> impl Responder {
    match db::add_meeting_room(&db_pool, item.into_inner()).await {
        Ok(room) => HttpResponse::Ok().json(room),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/meeting_room")]
pub async fn list_meeting_rooms(db_pool: web::Data<SqlitePool>) -> impl Responder {
    match db::list_meeting_rooms(&db_pool).await {
        Ok(rooms) => HttpResponse::Ok().json(rooms),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[put("/meeting_room/{id}")]
pub async fn update_meeting_room(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    item: web::Json<MeetingRoom>,
) -> impl Responder {
    let id = path.into_inner();
    match db::update_meeting_room(&db_pool, id, item.into_inner()).await {
        Ok(room) => HttpResponse::Ok().json(room),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[delete("/meeting_room/{id}")]
pub async fn delete_meeting_room(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let id = path.into_inner();
    match db::delete_meeting_room(&db_pool, id).await {
        Ok(true) => HttpResponse::Ok().json(json!({ "message": "Meeting room deleted" })),
        Ok(false) => HttpResponse::NotFound().json(json!({ "error": "Meeting room not found" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[post("/meeting_agenda")]
pub async fn create_meeting_agenda(
    db_pool: web::Data<SqlitePool>,
    session: Session,
    item: web::Json<MeetingAgenda>,
) -> impl Responder {
    let mut agenda = item.into_inner();
    let permission: i64 = session.get::<i64>("permissions").ok().flatten().unwrap_or(0);
    agenda.confirm = if permission & PERMISSION_MEETING_MANAGER != 0 { 1 } else { 0 };

    // Check conflict before insertion
    match db::check_meeting_conflict(&db_pool, &agenda).await {
        Ok(Some(conflict)) => {
            return HttpResponse::Conflict().json(json!({
                "error": "Time conflict with existing agenda",
                "conflict": conflict
            }))
        }
        Ok(None) => {} // Proceed
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({ "error": e.to_string() }));
        }
    }

    match db::add_meeting_agenda(&db_pool, agenda).await {
        Ok(record) => HttpResponse::Ok().json(record),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/meeting_agenda/room/{id}")]
pub async fn list_meeting_agendas(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    match db::list_meeting_agendas(&db_pool, path.into_inner()).await {
        Ok(agendas) => HttpResponse::Ok().json(agendas),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/meeting_agenda/{id}")]
pub async fn get_meeting_agenda(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    match db::get_meeting_agenda_by_id(&db_pool, path.into_inner()).await {
        Ok(agenda) => HttpResponse::Ok().json(agenda),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

pub async fn check_meeting_perm(
    db_pool: &web::Data<SqlitePool>,
    session: &Session,
    agenda_id: i64,
) -> Result<(), HttpResponse> {
    let permission: i64 = session.get::<i64>("permissions").ok().flatten().unwrap_or(0);
    if permission & (PERMISSION_MEETING_MANAGER | PERMISSION_ADMIN) != 0 {
        return Ok(())
    }
    let user: String = session.get::<String>("user_id").ok().flatten().unwrap_or("".to_string());
    match db::get_meeting_agenda_by_id(db_pool, agenda_id).await {
        Ok(agenda) => {
            if (agenda.confirm == 1) || (agenda.userid != user) {
                return Err(HttpResponse::Unauthorized().json(json!({ "error": "Unauthorized" })))
            }
            return Ok(())
        },
        Err(e) => return Err(HttpResponse::InternalServerError().json(json!({ "error": e.to_string() }))),
    }
}

#[put("/meeting_agenda/{id}")]
pub async fn update_meeting_agenda(
    db_pool: web::Data<SqlitePool>,
    session: Session,
    path: web::Path<i64>,
    item: web::Json<MeetingAgenda>,
) -> impl Responder {
    let agenda = item.into_inner();
    let agenda_id = path.into_inner();
    if let Err(e) = check_meeting_perm(&db_pool, &session, agenda_id).await {
        return e;
    }
    match db::update_meeting_agenda(&db_pool, agenda_id, agenda).await {
        Ok(updated) => HttpResponse::Ok().json(updated),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[delete("/meeting_agenda/{id}")]
pub async fn delete_meeting_agenda(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    match db::delete_meeting_agenda(&db_pool, path.into_inner()).await {
        Ok(true) => HttpResponse::Ok().json(json!({ "message": "Meeting agenda deleted" })),
        Ok(false) => HttpResponse::NotFound().json(json!({ "error": "Meeting agenda not found" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[put("/meeting_agenda/{id}/confirm")]
pub async fn confirm_meeting_agenda(
    db_pool: web::Data<SqlitePool>,
    session: Session,
    path: web::Path<i64>,
) -> impl Responder {
    let id = path.into_inner();
    let permission: i64 = session.get::<i64>("permissions").ok().flatten().unwrap_or(0);

    if permission & PERMISSION_MEETING_MANAGER == 0 {
        return HttpResponse::Forbidden().json(json!({ "error": "Permission denied" }));
    }

    match db::confirm_meeting_agenda(&db_pool, id).await {
        Ok(agenda) => HttpResponse::Ok().json(agenda),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

pub fn init_meeting_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_meeting_room)
       .service(update_meeting_room)
       .service(delete_meeting_room);
}

pub fn init_agenda_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_meeting_agenda)
       .service(list_meeting_agendas)
       .service(list_meeting_rooms)
       .service(get_meeting_agenda)
       .service(update_meeting_agenda)
       .service(confirm_meeting_agenda)
       .service(delete_meeting_agenda);
}

