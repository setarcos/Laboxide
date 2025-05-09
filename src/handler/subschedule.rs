use actix_session::Session;
use actix_web::{get, post, put, delete, web, HttpResponse, Responder};
use serde_json::json;
use sqlx::SqlitePool;

use crate::db;
use crate::models::SubSchedule;
use crate::utils::check_course_perm;

#[post("/subschedule")]
pub async fn create_subschedule(
    db_pool: web::Data<SqlitePool>,
    item: web::Json<SubSchedule>,
    session: Session,
) -> impl Responder {
    let sub = item.into_inner();
    if let Ok(schedule) = db::get_schedule_by_id(&db_pool, sub.schedule_id).await {
        if let Err(err) = check_course_perm(&db_pool, &session, schedule.course_id).await {
            return err;
        }
    } else {
        return HttpResponse::NotFound().json(json!({ "error": "Schedule not found" }));
    }
    match db::add_subschedule(&db_pool, sub).await {
        Ok(rec) => HttpResponse::Ok().json(rec),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/subschedule/{id}")]
pub async fn get_subschedule(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    match db::get_subschedule_by_id(&db_pool, path.into_inner()).await {
        Ok(rec) => HttpResponse::Ok().json(rec),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/subschedules/{schedule_id}")]
pub async fn list_subschedules(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    match db::list_subschedules(&db_pool, path.into_inner()).await {
        Ok(recs) => HttpResponse::Ok().json(recs),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[put("/subschedule/{id}")]
pub async fn update_subschedule(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    item: web::Json<SubSchedule>,
) -> impl Responder {
    match db::update_subschedule(&db_pool, path.into_inner(), item.into_inner()).await {
        Ok(rec) => HttpResponse::Ok().json(rec),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[delete("/subschedule/{id}")]
pub async fn delete_subschedule(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    session: Session,
) -> impl Responder {
    let sub_id = path.into_inner();
    if let Ok(schedule) = db::get_schedule_by_id(&db_pool, sub_id).await {
        if let Err(err) = check_course_perm(&db_pool, &session, schedule.course_id).await {
            return err;
        }
    } else {
        return HttpResponse::NotFound().json(json!({ "error": "Schedule not found" }));
    }
    match db::delete_subschedule(&db_pool, sub_id).await {
        Ok(true) => HttpResponse::Ok().json(json!({ "message": "SubSchedule deleted" })),
        Ok(false) => HttpResponse::NotFound().json(json!({ "error": "SubSchedule not found" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

pub fn init_subschedule_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_subschedule)
        .service(get_subschedule)
        .service(update_subschedule)
        .service(delete_subschedule);
}
