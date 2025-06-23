use actix_session::Session;
use actix_web::{get, post, put, delete, web, HttpResponse, Responder};
use serde_json::json;
use sqlx::SqlitePool;
use crate::db;
use crate::models::Equipment;

#[post("/equipment")]
pub async fn create_equipment(
    db_pool: web::Data<SqlitePool>,
    item: web::Json<Equipment>,
    session: Session,
) -> impl Responder {
    let user_id = match session.get::<String>("user_id") {
        Ok(Some(uid)) => uid,
        _ => return HttpResponse::Unauthorized().json(json!({ "error": "Not logged in" })),
    };

    let mut equipment = item.into_inner();
    equipment.owner_id = user_id;

    match db::add_equipment(&db_pool, equipment).await {
        Ok(equipment) => HttpResponse::Ok().json(equipment),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/equipment")]
pub async fn list_equipments(
    db_pool: web::Data<SqlitePool>,
    session: Session,
    web::Query(paging): web::Query<PaginationParams>,
) -> impl Responder {
    let user_id = match session.get::<String>("user_id") {
        Ok(Some(uid)) => uid,
        _ => return HttpResponse::Unauthorized().json(json!({ "error": "Not logged in" })),
    };

    let offset = (paging.page.unwrap_or(1) - 1).max(0) * paging.page_size.unwrap_or(10);
    let limit = paging.page_size.unwrap_or(10);

    match db::list_equipments(&db_pool, &user_id, offset as i64, limit as i64).await {
        Ok(equipments) => HttpResponse::Ok().json(equipments),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/equipment/{id}")]
pub async fn get_equipment(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let id = path.into_inner();
    match db::get_equipment_by_id(&db_pool, id).await {
        Ok(Some(equipment)) => HttpResponse::Ok().json(equipment),
        Ok(None) => HttpResponse::NotFound().json(json!({ "error": "Equipment not found" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[put("/equipment/{id}")]
pub async fn update_equipment(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    item: web::Json<Equipment>,
    session: Session,
) -> impl Responder {
    let id = path.into_inner();
    let user_id = match session.get::<String>("user_id") {
        Ok(Some(uid)) => uid,
        _ => return HttpResponse::Unauthorized().json(json!({ "error": "Not logged in" })),
    };

    let mut equipment = item.into_inner();
    equipment.owner_id = user_id.clone();

    match db::update_equipment(&db_pool, id, equipment).await {
        Ok(Some(equipment)) => HttpResponse::Ok().json(equipment),
        Ok(None) => HttpResponse::NotFound().json(json!({ "error": "Equipment not found" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[delete("/equipment/{id}")]
pub async fn delete_equipment(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    session: Session,
) -> impl Responder {
    let id = path.into_inner();
    let user_id = match session.get::<String>("user_id") {
        Ok(Some(uid)) => uid,
        _ => return HttpResponse::Unauthorized().json(json!({ "error": "Not logged in" })),
    };

    match db::delete_equipment(&db_pool, id, &user_id).await {
        Ok(true) => HttpResponse::Ok().json(json!({ "message": "Equipment deleted" })),
        Ok(false) => HttpResponse::NotFound().json(json!({ "error": "Equipment not found" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct PaginationParams {
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

pub fn init_equipment_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_equipment)
        .service(list_equipments)
        .service(get_equipment)
        .service(update_equipment)
        .service(delete_equipment);
}
