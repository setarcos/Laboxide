use actix_multipart::Multipart;
use actix_web::{get, post, web, delete, HttpResponse, Responder, HttpRequest};
use actix_session::Session;
use actix_files::NamedFile;
use futures_util::TryStreamExt;
use serde_json::json;
use sqlx::SqlitePool;
use std::fs::{File, metadata};
use std::io::Write;
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::utils::check_course_perm;
use crate::db;

#[post("/coursefile/upload")]
pub async fn upload_course_file(
    db_pool: web::Data<SqlitePool>,
    mut payload: Multipart,
    session: Session,
) -> impl Responder {
    use std::fs;

    let mut fname = None;
    let mut finfo = None;
    let mut course_id = None;
    let mut file_bytes = vec![];

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let name = content_disposition.get_name().unwrap_or_default();

        if name == "file" {
            let original_filename = content_disposition.get_filename().unwrap_or("unnamed").to_string();
            fname = Some(original_filename.clone());

            while let Some(chunk) = field.try_next().await.unwrap() {
                file_bytes.extend_from_slice(&chunk);
            }
        } else if name == "finfo" {
            let data = field.try_next().await.unwrap().unwrap();
            finfo = Some(String::from_utf8_lossy(&data).to_string());
        } else if name == "course_id" {
            let data = field.try_next().await.unwrap().unwrap();
            course_id = Some(String::from_utf8_lossy(&data).parse::<i64>().unwrap());
        }
    }

    if let Some(cid) = course_id {
        if let Err(err) = check_course_perm(&db_pool, &session, cid).await {
            return err;
        }
    }

    match (fname, finfo, course_id) {
        (Some(fname), Some(finfo), Some(course_id)) => {
            // Save file to ./uploads/courses/<course_id>/<fname>
            let upload_dir = format!("uploads/courses/{}", course_id);
            fs::create_dir_all(&upload_dir).unwrap();

            let file_path = format!("{}/{}", upload_dir, fname);
            let mut f = File::create(&file_path).unwrap();
            f.write_all(&file_bytes).unwrap();

            // Save metadata to DB
            match db::add_course_file(&db_pool, &fname, &finfo, course_id).await {
                Ok(record) => HttpResponse::Ok().json(record),
                Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
            }
        }
        _ => HttpResponse::BadRequest().json(json!({ "error": "Missing required fields" })),
    }
}

#[get("/coursefile/download/{id}")]
pub async fn download_course_file(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    req: HttpRequest,
) -> impl Responder {
    let id = path.into_inner();

    match db::get_course_file_by_id(&db_pool, id).await {
        Ok(course_file) => {
            let file_path = format!("uploads/courses/{}/{}", course_file.course_id, course_file.fname);
            match NamedFile::open_async(file_path).await {
                Ok(named_file) => named_file
                    .set_content_disposition(actix_web::http::header::ContentDisposition {
                        disposition: actix_web::http::header::DispositionType::Attachment,
                        parameters: vec![actix_web::http::header::DispositionParam::Filename(
                            course_file.fname.clone(),
                        )],
                    })
                    .into_response(&req),
                Err(_) => HttpResponse::NotFound().json(json!({ "error": "File not found on disk" })),
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[derive(Debug, Serialize)]
pub struct CourseFileResponse {
    pub id: i64,
    pub fname: String,
    pub finfo: String,
    pub course_id: i64,
    pub modified_time: Option<String>,
}

#[get("/coursefile/list/{id}")]
pub async fn list_course_files(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let course_id = path.into_inner();
    match db::list_course_files(&db_pool, course_id).await {
        Ok(files_from_db) => {
            let mut response_list = Vec::new();

            // Iterate over the files retrieved from the database
            for file_record in files_from_db {
                let file_path = format!(
                    "uploads/courses/{}/{}",
                    file_record.course_id, file_record.fname
                );

                // Try to get file metadata and its modified time
                let modified_time = match metadata(&file_path) {
                    Ok(metadata) => {
                        // If metadata is found, get the modified time
                        metadata.modified().map_or(None, |sys_time| {
                            // Convert SystemTime to a chrono DateTime object
                            let datetime: DateTime<Utc> = sys_time.into();
                            // Format it as an ISO 8601 string
                            Some(datetime.to_rfc3339())
                        })
                    }
                    Err(_) => None, // If metadata fails (e.g., file not found), return None
                };

                // Build the new response object
                response_list.push(CourseFileResponse {
                    id: file_record.id,
                    fname: file_record.fname,
                    finfo: file_record.finfo,
                    course_id: file_record.course_id,
                    modified_time, // Add the modified time here
                });
            }

            HttpResponse::Ok().json(response_list)
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[delete("/coursefile/{id}")]
pub async fn delete_course_file(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    session: Session,
) -> impl Responder {
    let id = path.into_inner();

    match db::get_course_file_by_id(&db_pool, id).await {
        Ok(file) => {
            if let Err(err) = check_course_perm(&db_pool, &session, file.course_id).await {
                return err;
            }
            let file_path = format!("uploads/courses/{}/{}", file.course_id, file.fname);
            let _ = std::fs::remove_file(&file_path); // Ignore error if file doesn't exist

            match db::delete_course_file(&db_pool, id).await {
                Ok(true) => HttpResponse::Ok().json(json!({ "message": "Course file deleted" })),
                Ok(false) => HttpResponse::NotFound().json(json!({ "error": "Course file not found" })),
                Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

pub fn init_course_file_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(upload_course_file)
        .service(delete_course_file);
}
