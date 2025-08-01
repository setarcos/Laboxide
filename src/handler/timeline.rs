use actix_session::Session;
use actix_web::{post, delete, get, web, HttpResponse, Responder, HttpRequest};
use actix_multipart::Multipart;
use actix_files::NamedFile;
use futures_util::TryStreamExt;
use serde_json::json;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;

use crate::config::{PERMISSION_TEACHER, PERMISSION_ADMIN, PERMISSION_STUDENT};
use crate::models::StudentTimeline;
use crate::db;

#[post("/timeline")]
pub async fn create_timeline(
    db_pool: web::Data<SqlitePool>,
    mut payload: Multipart,
    session: Session,
) -> impl Responder {
    let mut stu_id = None;
    let mut tea_id = None;
    let mut schedule_id = None;
    let mut subschedule = None;
    let mut subcourse_id = None;
    let mut note_type = None;

    let mut note_filename = None;
    let mut file_bytes = vec![];
    let user_id: String = session.get::<String>("user_id").ok().flatten().unwrap_or_default();
    let permission: i64 = session.get::<i64>("permissions").ok().flatten().unwrap_or(0);

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
            "tea_id" => {
                let data = field.try_next().await.unwrap().unwrap();
                tea_id = Some(String::from_utf8_lossy(&data).to_string());
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
            "note" => {
                if note_type.unwrap_or(0) != 1 {
                    let data = field.try_next().await.unwrap().unwrap();
                    note_filename = Some(String::from_utf8_lossy(&data).to_string());
                }
            }
            _ => {}
        }
    }
    if let (Some(stu_id), Some(schedule_id)) = (&stu_id, schedule_id) {
        if (permission & PERMISSION_TEACHER == 0) && stu_id != &user_id {
            return HttpResponse::Unauthorized().json(json!({ "error": "Unauthorized" }));
        }
        if let Ok(count) = db::count_student_timeline_entries(&db_pool, &stu_id, schedule_id).await {
            if count > 100 {
                return HttpResponse::Unauthorized().json(json!({ "error": "Too many entries." }));
            }
        }
    } else {
        return HttpResponse::BadRequest().json(json!({ "error": "Missing required parameters" }));
    }
    // Save file if it's a file note
    if note_type == Some(1) && note_filename.is_some() && stu_id.is_some() && subcourse_id.is_some() {
        let upload_dir = format!("uploads/coursetl/{}/{}", subcourse_id.as_ref().unwrap(), stu_id.as_ref().unwrap());
        fs::create_dir_all(&upload_dir).unwrap();

        let original_name = note_filename.as_ref().unwrap();
        let mut file_path = PathBuf::from(&upload_dir);
        file_path.push(original_name);

        let mut final_filename = original_name.clone();
        let mut counter = 1;

        while file_path.exists() {
            let stem = Path::new(original_name).file_stem().unwrap().to_string_lossy();
            let ext = Path::new(original_name).extension().and_then(|e| Some(format!(".{}", e.to_string_lossy()))).unwrap_or_default();
            final_filename = format!("{}({}){}", stem, counter, ext);
            file_path = PathBuf::from(&upload_dir);
            file_path.push(&final_filename);
            counter += 1;
        }

        let mut f = std::fs::File::create(&file_path).unwrap();
        f.write_all(&file_bytes).unwrap();

        // Update the filename to the final one used
        note_filename = Some(final_filename);
    }

    match (
        stu_id,
        tea_id,
        schedule_id,
        subschedule,
        subcourse_id,
        note_filename,
        note_type,
    ) {
        (
            Some(stu_id),
            Some(tea_id),
            Some(schedule_id),
            Some(subschedule),
            Some(subcourse_id),
            Some(note),
            Some(note_type),
        ) => {
            let new_timeline = StudentTimeline {
                id: 0,
                stu_id,
                tea_id,
                schedule_id,
                subschedule,
                subcourse_id,
                note,
                notetype: note_type,
                timestamp: chrono::Local::now().naive_local(),
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
    session: &Session,
) -> Result<StudentTimeline, HttpResponse> {
    let timeline = db::get_timeline_by_id(db_pool, id).await.map_err(|e| {
        HttpResponse::InternalServerError().json(json!({ "error": e.to_string() }))
    })?;

    let permission: i64 = session.get::<i64>("permissions").ok().flatten().unwrap_or(0);
    let user_id: Option<String> = session.get("user_id").unwrap_or(None);

    let is_student = user_id.as_deref() == Some(&timeline.stu_id);
    let is_teacher = user_id.as_deref() == Some(&timeline.tea_id);
    let is_admin = permission & PERMISSION_ADMIN != 0;

    if is_student {
        if let Ok(Some(log)) = db::get_student_log_by_schedule(&db_pool, &timeline.stu_id, timeline.schedule_id).await {
            if log.confirm == 1 {
                return Err(HttpResponse::Unauthorized().json(json!({ "error": "Can't delete after confirmation." })));
            }
        }
    }

    if !(is_student || is_teacher || is_admin) {
        return Err(HttpResponse::Unauthorized().json(json!({ "error": "Unauthorized" })));
    }

    Ok(timeline)
}

#[delete("/timeline/{id}")]
pub async fn delete_timeline(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    session: Session,
) -> impl Responder {
    let id = path.into_inner();

    match check_timeline_permission(&db_pool, id, &session).await {
        Ok(timeline) => {
            if timeline.notetype == 1 {
                let file_path = format!("uploads/coursetl/{}/{}/{}", timeline.subcourse_id, timeline.stu_id, timeline.note);
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
    session: Session,
) -> impl Responder {
    let (subcourse_id, stu_id) = path.into_inner();
    let mut tea_id = "-".to_string();
    let user_id: String = session.get::<String>("user_id").ok().flatten().unwrap_or_default();
    let permission: i64 = session.get::<i64>("permissions").ok().flatten().unwrap_or(0);
    if permission & PERMISSION_TEACHER != 0 {
        tea_id = user_id.clone();
    }
    if (permission & PERMISSION_STUDENT != 0) && user_id != stu_id {
        return HttpResponse::Unauthorized().json(json!({ "error": "Unauthorized" }));
    }
    match db::list_timelines_by_student(&db_pool, subcourse_id, &stu_id, &tea_id).await {
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
        Ok(entry) if entry.notetype == 1 => {
            let file_path = format!("uploads/coursetl/{}/{}/{}", entry.subcourse_id, entry.stu_id, entry.note);
            match NamedFile::open_async(&file_path).await {
                Ok(file) => file.into_response(&req),
                Err(_) => HttpResponse::NotFound().body("File not found"),
            }
        }
        Ok(_) => HttpResponse::BadRequest().json(json!({"error": "This entry does not contain a file."})),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": e.to_string()})),
    }
}

pub fn init_timeline_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_timeline)
       .service(list_timelines_by_student)
       .service(download_timeline_file)
       .service(delete_timeline);
}
