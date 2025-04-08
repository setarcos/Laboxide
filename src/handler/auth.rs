// src/handler/auth.rs
use actix_web::{get, HttpResponse, Responder, web};
use actix_session::Session;
use serde_json::json;
use crate::db;
use crate::config::{Config};
use log::error;

#[derive(serde::Deserialize)]
pub struct LoginQuery {
    pub user_id: String,
    pub token: String,
}

// Login Route - authenticate user by token
#[get("/auth")]
pub async fn login(
    session: Session,
    query: web::Query<LoginQuery>,
    config: web::Data<Config>,
    db_pool: web::Data<sqlx::SqlitePool>,
) -> impl Responder {
    if query.token != config.token_secret {
        return HttpResponse::Unauthorized().json(json!({ "error": "Invalid token" }));
    }

    // Check if the user exists in the database
    match db::get_user_by_id(&db_pool, &query.user_id).await {
        Ok(Some(user)) => {
            // Store user_id and permission in the session
            session.insert("user_id", &user.user_id).unwrap();
            session.insert("permissions", &user.permission).unwrap();
            session.insert("realname", &user.username).unwrap();
            HttpResponse::Ok().json(user)
        },
        Ok(None) => {
            // Store user_id and permission as browse-only (default permission 0)
            session.insert("user_id", &query.user_id).unwrap();
            session.insert("permissions", &0i64).unwrap();  // Browse-only, no permissions
            session.insert("realname", "Guest").unwrap();
            HttpResponse::Ok().json(json!({ "message": "User found as browse-only" }))
        },
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

// Logout Route - clear session
#[get("/logout")]
pub async fn logout(session: Session) -> impl Responder {
    session.purge();
    HttpResponse::Ok().json(json!({ "message": "Logged out" }))
}

// Greet Route - greet logged-in user
#[get("/greet")]
pub async fn greet(session: Session) -> impl Responder {
    let user_id_res = session.get::<String>("user_id");
    let realname_res = session.get::<String>("realname");
    let permissions_res = session.get::<i64>("permissions");

    match (user_id_res, realname_res, permissions_res) {
        // All keys retrieved successfully (Ok) and contain values (Some)
        (Ok(Some(user_id)), Ok(Some(realname)), Ok(Some(permissions))) => {
            HttpResponse::Ok().json(json!({
                "user_id": user_id,
                "realname": realname,
                "permissions": permissions,
            }))
        }

        // --- Error Case ---
        (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
            error!("Session get error: {:?}", e);
            // Return a generic server error to the client
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to retrieve session data"
            }))
        }

        // --- Unauthorized Case ---
        _ => {
            HttpResponse::Unauthorized().json(json!({
                "error": "Not logged in or session data incomplete"
            }))
        }
    }
}

// Register the authentication routes
pub fn init_auth_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(login)
       .service(logout)
       .service(greet);
}

