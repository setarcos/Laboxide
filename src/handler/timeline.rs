use actix_web::{post, put, delete, get, web, HttpResponse, Responder, HttpRequest};
use actix_multipart::Multipart;
use actix_files::NamedFile;
use futures_util::TryStreamExt;
use serde_json::json;
use sqlx::SqlitePool;
use std::fs;
use std::io::Write;
use chrono::NaiveDateTime;

use crate::models::StudentTimeline;
use crate::db;

#[post("/timeline")]
pub async fn create_timeline(
    db_pool: web::Data<SqlitePool>,
    mut payload: Multipart,
) -> impl Responder {
    let mut stu_id = None;
    let mut tea_name = None;
    let mut schedule_id = None;
    let mut subschedule = None;
    let mut subcourse_id = None;
    let mut note_type = None;
    let mut timestamp: Option<NaiveDateTime> = None;

    let mut note_filename = None;
    let mut file_bytes = vec![];

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let name = content_disposition.get_name().unwrap_or_default();

        match name {
            "file" => {
                let original_filename = content_disposition.get_filename().unwrap_or("unnamed").to_string();
                note_filename = Some(original_filename.clone());

                while let Some(chunk) = field.try_next().await.unwrap() {
                    file_bytes.extend_from_slice(&chunk);
                }
            }
            "stu_id" => {
                let data = field.try_next().await.unwrap().unwrap();
                stu_id = Some(String::from_utf8_lossy(&data).to_string());
            }
            "tea_name" => {
                let data = field.try_next().await.unwrap().unwrap();
                tea_name = Some(String::from_utf8_lossy(&data).to_string());
            }
            "schedule_id" => {
                let data = field.try_next().await.unwrap().unwrap();
                schedule_id = Some(String::from_utf8_lossy(&data).parse::<i64>().unwrap_or(0));
            }
            "subschedule" => {
                let data = field.try_next().await.unwrap().unwrap();
                subschedule = Some(String::from_utf8_lossy(&data).to_string());
            }
            "subcourse_id" => {
                let data = field.try_next().await.unwrap().unwrap();
                subcourse_id = Some(String::from_utf8_lossy(&data).parse::<i64>().unwrap_or(0));
            }
            "notetype" => {
                let data = field.try_next().await.unwrap().unwrap();
                note_type = Some(String::from_utf8_lossy(&data).parse::<i64>().unwrap_or(0));
            }
            "timestamp" => {
                let data = field.try_next().await.unwrap().unwrap();
                timestamp = NaiveDateTime::parse_from_str(&String::from_utf8_lossy(&data), "%Y-%m-%d %H:%M:%S").ok();
            }
            "note" => {
                if note_type.unwrap_or(0) != 1 {
                    let data = field.try_next().await.unwrap().unwrap();
                    note_filename = Some(String::from_utf8_lossy(&data).to_string());
                }
            }
            _ => {}
        }
    }

    // Save file if it's a file note
    if note_type == Some(1) && note_filename.is_some() && stu_id.is_some() {
        let upload_dir = format!("uploads/courses/{}", stu_id.as_ref().unwrap());
        fs::create_dir_all(&upload_dir).unwrap();

        let file_path = format!("{}/{}", upload_dir, note_filename.as_ref().unwrap());
        let mut f = std::fs::File::create(&file_path).unwrap();
        f.write_all(&file_bytes).unwrap();
    }

    match (
        stu_id,
        tea_name,
        schedule_id,
        subschedule,
        subcourse_id,
        note_filename,
        note_type,
        timestamp,
    ) {
        (
            Some(stu_id),
            Some(tea_name),
            Some(schedule_id),
            Some(subschedule),
            Some(subcourse_id),
            Some(note),
            Some(note_type),
            Some(timestamp),
        ) => {
            let new_timeline = StudentTimeline {
                id: 0,
                stu_id,
                tea_name,
                schedule_id,
                subschedule,
                subcourse_id,
                note,
                notetype: note_type,
                timestamp,
            };

            match db::add_student_timeline(&db_pool, new_timeline).await {
                Ok(record) => HttpResponse::Ok().json(record),
                Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
            }
        }
        _ => HttpResponse::BadRequest().json(json!({ "error": "Missing or invalid fields" })),
    }
}

async fn check_timeline_permission(
    db_pool: &SqlitePool,
    id: i64,
    session: &actix_session::Session,
) -> Result<StudentTimeline, HttpResponse> {
    let timeline = db::get_timeline_by_id(db_pool, id).await.map_err(|e| {
        HttpResponse::InternalServerError().json(json!({ "error": e.to_string() }))
    })?;

    let timeline = match timeline {
        Some(t) => t,
        None => return Err(HttpResponse::NotFound().json(json!({ "error": "Timeline not found" }))),
    };

    let user_id: Option<String> = session.get("user_id").unwrap_or(None);
    let realname: Option<String> = session.get("realname").unwrap_or(None);

    let is_student = user_id.as_deref() == Some(&timeline.stu_id);
    let is_teacher = realname.as_deref() == Some(&timeline.tea_name);

    if !(is_student || is_teacher) {
        return Err(HttpResponse::Unauthorized().json(json!({ "error": "Unauthorized" })));
    }

    Ok(timeline)
}

#[put("/timeline/{id}")]
pub async fn update_timeline(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    item: web::Json<StudentTimeline>,
    session: actix_session::Session,
) -> impl Responder {
    let id = path.into_inner();

    match check_timeline_permission(&db_pool, id, &session).await {
        Ok(_) => {
            match db::update_student_timeline(&db_pool, id, item.into_inner()).await {
                Ok(Some(updated)) => HttpResponse::Ok().json(updated),
                Ok(None) => HttpResponse::NotFound().json(json!({ "error": "Timeline not found" })),
                Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
            }
        }
        Err(resp) => resp,
    }
}

#[delete("/timeline/{id}")]
pub async fn delete_timeline(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    session: actix_session::Session,
) -> impl Responder {
    let id = path.into_inner();

    match check_timeline_permission(&db_pool, id, &session).await {
        Ok(timeline) => {
            if timeline.notetype == 1 {
                let file_path = format!("uploads/courses/{}/{}", timeline.stu_id, timeline.note);
                let _ = std::fs::remove_file(&file_path);
            }

            match db::delete_student_timeline(&db_pool, id).await {
                Ok(true) => HttpResponse::Ok().json(json!({ "message": "Timeline deleted" })),
                Ok(false) => HttpResponse::NotFound().json(json!({ "error": "Timeline not found" })),
                Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
            }
        }
        Err(resp) => resp,
    }
}

#[get("/timeline/schedule/{subcourse_id}/{schedule_id}")]
pub async fn list_timelines_by_schedule(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<(i64, i64)>,
) -> impl Responder {
    let (subcourse_id, schedule_id) = path.into_inner();
    match db::list_timelines_by_schedule(&db_pool, subcourse_id, schedule_id).await {
        Ok(items) => HttpResponse::Ok().json(items),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/timeline/student/{subcourse_id}/{stu_id}")]
pub async fn list_timelines_by_student(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<(i64, String)>,
) -> impl Responder {
    let (subcourse_id, stu_id) = path.into_inner();
    match db::list_timelines_by_student(&db_pool, subcourse_id, stu_id).await {
        Ok(items) => HttpResponse::Ok().json(items),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/timeline/file/{id}")]
pub async fn download_timeline_file(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    req: HttpRequest,
) -> impl Responder {
    let id = path.into_inner();

    match db::get_timeline_by_id(&db_pool, id).await {
        Ok(Some(entry)) if entry.notetype == 1 => {
            let file_path = format!("uploads/courses/{}/{}", entry.stu_id, entry.note);
            match NamedFile::open_async(&file_path).await {
                Ok(file) => file.into_response(&req),
                Err(_) => HttpResponse::NotFound().body("File not found"),
            }
        }
        Ok(Some(_)) => HttpResponse::BadRequest().body("This entry does not contain a file."),
        Ok(None) => HttpResponse::NotFound().body("Timeline entry not found."),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": e.to_string()})),
    }
}

pub fn init_timeline_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(update_timeline)
       .service(create_timeline)
       .service(list_timelines_by_student)
       .service(download_timeline_file)
       .service(delete_timeline);
}
