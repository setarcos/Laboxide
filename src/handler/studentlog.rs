use actix_web::{post, put, web, get, HttpResponse, Responder};
use actix_session::Session;
use serde_json::json;
use sqlx::SqlitePool;
use serde::Deserialize;
use crate::db;
use crate::models::StudentLog;
use chrono::NaiveDateTime;

pub fn check_stu_id(
    session: &Session,
    stu_id: &String,
) -> Result<(), HttpResponse> {
    let user_id: String = session.get::<String>("user_id").ok().flatten().unwrap_or_default();
    if user_id != *stu_id {
        return Err(HttpResponse::Forbidden().json(json!({"error": "Can't use a different ID"})));
    }
    Ok(())
}

#[post("/student_log")]
pub async fn create_student_log(
    db_pool: web::Data<SqlitePool>,
    item: web::Json<StudentLog>,
    session: Session,
) -> impl Responder {
    let mut log = item.into_inner();
    if let Err(err) = check_stu_id(&session, &log.stu_id) {
        return err;
    }
    log.confirm = 0; // make sure it's not confirmed.
    match db::add_student_log(&db_pool, log).await {
        Ok(log) => HttpResponse::Ok().json(log),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[put("/student_log/{id}")]
pub async fn update_student_log(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    item: web::Json<StudentLog>,
    session: Session,
) -> impl Responder {
    let id = path.into_inner();
    let newlog = item.into_inner();
    if let Err(err) = check_stu_id(&session, &newlog.stu_id) {
        return err;
    }
    if let Ok(log) = db::get_student_log_by_id(&db_pool, id).await {
        if newlog.stu_id != log.stu_id {
            return HttpResponse::Forbidden().json(json!({"error": "Can't use a different ID"}));
        }
        match db::update_student_log(&db_pool, id, newlog).await {
            Ok(()) => HttpResponse::Ok().json(json!({"status": "updated"})),
            Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
        }
    } else {
        return HttpResponse::Forbidden().json(json!({"error": "No log found."}));
    }
}

#[get("/student_log/recent/{subcourse_id}")]
pub async fn get_recent_logs(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let subcourse_id = path.into_inner();
    match db::list_recent_logs(&db_pool, subcourse_id).await {
        Ok(logs) => HttpResponse::Ok().json(logs),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string()})),
    }
}
#[derive(Debug, Deserialize)]
pub struct TimeRangeQuery {
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
}

#[post("/student_log/room/{room_id}")]
pub async fn get_student_logs_by_room(
    db_pool: web::Data<SqlitePool>,
    time_query: web::Json<TimeRangeQuery>,
    path: web::Path<i64>,
) -> impl Responder {
    let room_id = path.into_inner();
    match db::find_student_logs_by_room(
        &db_pool, room_id, time_query.start_time, time_query.end_time,
    ) .await {
        Ok(logs) => HttpResponse::Ok().json(logs),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[derive(Debug, Deserialize)]
pub struct TeacherConfirmRequest {
    pub tea_note: String,
}

#[put("/student_log/force/{subcourse_id}/{stu_id}")]
pub async fn force_student_log(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<(i64, String)>,
    session: Session,
) -> impl Responder {
    let (subcourse_id, stu_id) = path.into_inner();
    let realname: String = session.get::<String>("realname").ok().flatten().unwrap_or_default();
    if let Ok(mut log) = db::get_default_log(&db_pool, &stu_id, subcourse_id).await {
        if let Ok(stu_name) = db::get_student_name(&db_pool, &stu_id, subcourse_id).await {
            log.stu_name = stu_name;
        }
        log.confirm = 1;
        log.tea_name = realname;
        log.tea_note = "Log by T".to_string();
        match db::add_student_log(&db_pool, log).await {
            Ok(log) => HttpResponse::Ok().json(log),
            Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
        }
    } else {
        HttpResponse::InternalServerError().json(json!({ "error": "Create log failed." }))
    }
}

#[put("/student_log/confirm/{id}")]
pub async fn confirm_student_log(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    item: web::Json<TeacherConfirmRequest>,
    session: Session,
) -> impl Responder {
    let id = path.into_inner();
    let log = item.into_inner();
    let realname: String = session.get::<String>("realname").ok().flatten().unwrap_or_default();
    match db::confirm_student_log(&db_pool, id, &log.tea_note, &realname).await {
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
