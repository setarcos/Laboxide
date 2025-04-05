use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;

use crate::models::User;
use crate::db;

#[derive(Deserialize)]
pub struct UserData {
    pub user_id: String,
    pub username: String,
    pub permission: i64,
}

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

#[post("/user")]
pub async fn create_user(
    db_pool: web::Data<SqlitePool>,
    user_data: web::Json<UserData>,
) -> impl Responder {
    let user = User {
        user_id: user_data.user_id.clone(),
        username: user_data.username.clone(),
        permission: user_data.permission,
    };

    match db::add_user(&db_pool, &user).await {
        Ok(_) => HttpResponse::Created().json(json!({ "data": user })),
        Err(_) => HttpResponse::InternalServerError().json(json!({ "error": "Failed to create user" })),
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
        Err(_) => HttpResponse::InternalServerError().json(json!({ "error": "Failed to fetch user" })),
    }
}

#[get("/users")]
pub async fn list_users(
    db_pool: web::Data<SqlitePool>,
) -> impl Responder {
    match db::list_users(&db_pool).await {
        Ok(users) => HttpResponse::Ok().json(json!({ "data": users })),
        Err(_) => HttpResponse::InternalServerError().json(json!({ "error": "Failed to list users" })),
    }
}

#[put("/user/{user_id}")]
pub async fn update_user(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<String>,
    updated: web::Json<UserData>,
) -> impl Responder {
    let user = User {
        user_id: path.into_inner(),
        username: updated.username.clone(),
        permission: updated.permission,
    };

    match db::update_user(&db_pool, &user).await {
        Ok(_) => HttpResponse::Ok().json(json!({ "data": user })),
        Err(_) => HttpResponse::InternalServerError().json(json!({ "error": "Failed to update user" })),
    }
}

#[delete("/user/{user_id}")]
pub async fn delete_user(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<String>,
) -> impl Responder {
    let user_id = path.into_inner();
    match db::delete_user(&db_pool, &user_id).await {
        Ok(_) => HttpResponse::Ok().json(json!({ "message": "User deleted" })),
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

