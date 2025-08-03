// src/handler/auth.rs
use actix_web::{post, get, HttpResponse, Responder, web, HttpRequest};
use actix_session::Session;
use serde_json::json;
use serde::Deserialize;
use crate::db;
use crate::config::{PERMISSION_STUDENT, PERMISSION_TEACHER, Config};
use crate::models::User;
use log::error;
use std::env;

#[derive(serde::Deserialize)]
pub struct LoginQuery {
    pub token: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IaaaValidateResponse {
    pub err_code: String,
    #[serde(default)]
    pub user_info: Option<IaaaUserInfo>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IaaaUserInfo {
    pub identity_id: String,
    pub identity_type: String,
    pub name: String,
}

fn put_user_in_session(session: &actix_session::Session, user: &User) {
    session.insert("user_id", user.user_id.clone()).unwrap();
    session.insert("permissions", user.permission).unwrap();
    session.insert("realname", user.username.clone()).unwrap();
}

#[post("/auth")]
pub async fn iaaa_callback(
    req: HttpRequest,
    session: Session,
    config: web::Data<Config>,
    token_query: web::Json<LoginQuery>,
    db_pool: web::Data<sqlx::SqlitePool>,
) -> impl Responder {

    let token = token_query.into_inner().token;

    // --- Backdoor for Development ---
    // Don't put APP_ENV in Config, thus can open backdoor without restarting the server.
    let app_env = env::var("APP_ENV").unwrap_or_else(|_| "production".to_string());
    if app_env == "development" && (token.starts_with("Student") || token.starts_with("Teacher")) {
        log::info!("Using development backdoor for token: {}", token);
        let parts: Vec<&str> = token.split(',').collect();
        if parts[0] == "Student" {
            let user = User {
                user_id: parts[1].to_string(),
                username: if parts.len() > 2 {parts[2].to_string()} else {String::from("贾鸣")},
                permission: PERMISSION_STUDENT,
            };
            put_user_in_session(&session, &user);
            return HttpResponse::Ok().json(user);
        } else {
            match db::get_user_by_id(&db_pool, parts[1]).await {
                Ok(mut user) => {
                    user.permission |= PERMISSION_TEACHER;
                    put_user_in_session(&session, &user);
                    return HttpResponse::Ok().json(user);
                },
                Err(_) => {
                    let user = User {
                        user_id: parts[1].to_string(),
                        username: if parts.len() > 2 {parts[2].to_string()} else {String::from("贾诗")},
                        permission: PERMISSION_TEACHER,
                    };
                    put_user_in_session(&session, &user);
                    return HttpResponse::Ok().json(user);
                },
            };
        }

    }

    // Get client IP, respecting X-Forwarded-For header
    let ip = req.peer_addr()
        .map(|addr| addr.ip().to_string())
        .or_else(|| {
            req.headers()
                .get("X-Forwarded-For")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.split(',').next().unwrap().trim().to_string())
        })
        .unwrap_or_else(|| "0.0.0.0".to_string());

    log::info!("Validating token for IP: {}", ip);

    let para_to_hash = format!("appId={}&remoteAddr={}&token={}{}",
        &config.iaaa_id, ip, token, &config.iaaa_key);
    let digest = md5::compute(para_to_hash.as_bytes());
    let msg_abs = format!("{:x}", digest);

    let client = reqwest::Client::new();
    let url = "https://iaaa.pku.edu.cn/iaaa/svc/token/validate.do";
    let res = match client
        .get(url)
        .query(&[
            ("appId", &config.iaaa_id),
            ("remoteAddr", &ip),
            ("token", &token),
            ("msgAbs", &msg_abs),
        ])
        .send()
        .await {
            Ok(response) => response, // If successful, continue with the response
            Err(e) => {
                // If the request fails, log the error and return a 500 response
                log::error!("Failed to send request to IAAA: {}", e);
                return HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to contact authentication service"
                }));
            }
        };
    let validation_response = match res.json::<IaaaValidateResponse>().await {
        Ok(data) => data, // If successful, continue with the parsed data
        Err(e) => {
            // If JSON parsing fails, log the error and return a 500 response
            log::error!("Failed to parse JSON from IAAA: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "error": "Invalid response from authentication service"
            }));
        }
    };

    if validation_response.err_code != "0" {
        log::warn!("IAAA validation failed with code: {}", validation_response.err_code);
        return HttpResponse::InternalServerError().json(json!({ "error": "IAAA failed." }))
    }

    let user_info = match validation_response.user_info {
        Some(info) => info, // If user_info exists, assign it to the `user_info` variable.
        None => {
            // If user_info is None, log the error and return an appropriate HTTP response.
            log::error!("IAAA validation succeeded but did not return user info");
            // Since this function returns `impl Responder`, we can return an HttpResponse directly.
            return HttpResponse::BadRequest().json(json!({ "error": "IAAA did not return user info" }));
        }
    };
    log::info!("IAAA validation successful for user: {}", user_info.name);
    let mut user = User {
        user_id: user_info.identity_id,
        username: user_info.name,
        permission: 0,
    };
    // In a real app, you might look up the user in a DB here.
    let mut perm = if user_info.identity_type == "职工" { PERMISSION_TEACHER } else { PERMISSION_STUDENT };
    let db_user_result = db::get_user_by_id(&db_pool, &user.user_id).await;
    match db_user_result {
        Ok(db_user) => {
            perm |= db_user.permission;
        }
        Err(e) => {
            log::error!("Failed to fetch user {} from DB: {:?}", user.user_id, e);
        }
    }
    user.permission = perm;
    // Store the user info in the session
    put_user_in_session(&session, &user);
    HttpResponse::Ok().json(user)
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
    cfg.service(iaaa_callback)
       .service(logout)
       .service(greet);
}

