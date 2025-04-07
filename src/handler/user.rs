use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use serde_json::json;
use sqlx::SqlitePool;

use crate::models::User;
use crate::db;

#[post("/user")]
pub async fn create_user(
    db_pool: web::Data<SqlitePool>,
    item: web::Json<User>,
) -> impl Responder {

    match db::add_user(&db_pool, item.into_inner()).await {
        Ok(user) => HttpResponse::Created().json(user),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/user/{user_id}")]
pub async fn get_user(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> impl Responder {
    let user_id = path.into_inner();
    match db::get_user_by_id(&db_pool, &user_id).await {
        Ok(Some(user)) => HttpResponse::Ok().json(json!({ "data": user })),
        Ok(None) => HttpResponse::NotFound().json(json!({ "error": "User not found" })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[get("/user")]
pub async fn list_users(
    db_pool: web::Data<SqlitePool>,
) -> impl Responder {
    match db::list_users(&db_pool).await {
        Ok(users) => HttpResponse::Ok().json(json!({ "data": users })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[put("/user")]
pub async fn update_user(
    db_pool: web::Data<SqlitePool>,
    item: web::Json<User>,
) -> impl Responder {

    match db::update_user(&db_pool, item.into_inner()).await {
        Ok(user) => HttpResponse::Ok().json(user),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[delete("/user/{user_id}")]
pub async fn delete_user(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> impl Responder {
    let user_id = path.into_inner();
    match db::delete_user(&db_pool, &user_id).await {
        Ok(true) => HttpResponse::Ok().json(json!({ "message": "User deleted" })),
        Ok(false) => HttpResponse::NotFound().json(json!({ "error": "User not found" })),
        Err(_) => HttpResponse::InternalServerError().json(json!({ "error": "Failed to delete user" })),
    }
}

pub fn init_user_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_user)
        .service(get_user)
        .service(list_users)
        .service(update_user)
        .service(delete_user);
}

