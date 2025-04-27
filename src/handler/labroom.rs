use actix_web::{get, post, put, delete, web, HttpResponse, Responder};
use serde_json::json;
use sqlx::SqlitePool;

use crate::db;
use crate::models::Labroom;

#[post("/labroom")]
pub async fn create_labroom(
    db_pool: web::Data<SqlitePool>,
    item: web::Json<Labroom>,
) -> impl Responder {
    match db::add_labroom(&db_pool, item.into_inner()).await {
        Ok(labroom) => HttpResponse::Ok().json(labroom),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/labroom")]
pub async fn list_labrooms(
    db_pool: web::Data<SqlitePool>,
) -> impl Responder {
    match db::list_labrooms(&db_pool).await {
        Ok(labrooms) => HttpResponse::Ok().json(labrooms),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/labroom/{id}")]
pub async fn get_labroom(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let id = path.into_inner();
    match db::get_labroom_by_id(&db_pool, id).await {
        Ok(Some(labroom)) => HttpResponse::Ok().json(labroom),
        Ok(None) => HttpResponse::NotFound().json(json!({ "error": "Labroom not found" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[put("/labroom/{id}")]
pub async fn update_labroom(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    item: web::Json<Labroom>,
) -> impl Responder {
    let id = path.into_inner();
    match db::update_labroom(&db_pool, id, item.into_inner()).await {
        Ok(Some(labroom)) => HttpResponse::Ok().json(labroom),
        Ok(None) => HttpResponse::NotFound().json(json!({ "error": "Labroom not found" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[delete("/labroom/{id}")]
pub async fn delete_labroom(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let id = path.into_inner();
    match db::delete_labroom(&db_pool, id).await {
        Ok(true) => HttpResponse::Ok().json(json!({ "message": "Labroom deleted" })),
        Ok(false) => HttpResponse::NotFound().json(json!({ "error": "Labroom not found" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

pub fn init_labroom_adminroutes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_labroom)
        .service(update_labroom)
        .service(delete_labroom);
}
