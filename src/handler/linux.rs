use actix_session::Session;
use actix_web::{post, patch, web, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;
use std::process::Command;
use log::error;
use crate::config::{Config, PERMISSION_LINUX};
use rand::{distributions::Alphanumeric, Rng};

#[derive(Deserialize)]
pub struct SSHKeyPayload {
    sshkey: String,
}

#[post("/adduser")]
pub async fn add_linux_user(
    session: Session,
    config: web::Data<Config>,
    payload: web::Json<SSHKeyPayload>,
) -> impl Responder {
    let user_id = session.get::<String>("user_id").ok().flatten().unwrap_or_default();
    let permission: i64 = session.get::<i64>("permissions").ok().flatten().unwrap_or(0);

    if user_id.is_empty() || permission & PERMISSION_LINUX == 0 {
        return HttpResponse::Unauthorized().json(json!({ "error": "Permission denied!" }));
    }
    let sshkey = &payload.sshkey;

    // Build command
    let command_str = format!(
        "ssh -t {}@{} '/home/{}/manage_user.sh \"{}\" \"{}\"'",
        &config.remote_user,
        &config.remote_host,
        &config.remote_user,
        user_id,
        sshkey.replace('\"', "\\\"") // sanitize quotes
    );

    // Run command
    let result = Command::new("sh")
        .arg("-c")
        .arg(&command_str)
        .output();

    match result {
        Ok(output) => {
            if output.status.success() {
                HttpResponse::Ok().json(json!({ "status": "success" }))
            } else {
                error!(
                    "SSH command failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                HttpResponse::InternalServerError().json(json!({"error": "SSH command failed"}))
            }
        }
        Err(e) => {
            error!("Failed to execute command: {:?}", e);
            HttpResponse::InternalServerError().json(json!({ "error": "Execution failed" }))
        }
    }
}

/// Helper function to generate a random, URL-safe string of a given length.
fn generate_password(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// Handler to create a new user in Forgejo.
#[post("/gituser")]
pub async fn add_forgejo_user(
    session: Session,
    config: web::Data<Config>,
) -> impl Responder {
    // 1. Authentication & Authorization Check
    let user_id = match session.get::<String>("user_id") {
        Ok(Some(id)) if !id.is_empty() => id,
        _ => return HttpResponse::Unauthorized().json(json!({ "error": "Not authenticated" })),
    };
    let permission: i64 = session.get("permissions").unwrap_or_default().unwrap_or(0);
    if permission & PERMISSION_LINUX == 0 {
        return HttpResponse::Forbidden().json(json!({ "error": "Permission denied" }));
    }

    // 2. Prepare request data
    let password = generate_password(16);
    let forgejo_payload = json!({
        "username": user_id,
        "email": format!("{}@stu.pku.edu.cn", user_id),
        "password": password,
        // Add other fields as needed, e.g., "must_change_password": true
    });

    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/admin/users", config.forge_url);

    // 3. Send API request to Forgejo
    let res = client
        .post(&url)
        .bearer_auth(&config.forge_key)
        .json(&forgejo_payload)
        .send()
        .await;

    // 4. Handle response
    match res {
        Ok(response) => {
            if response.status() == reqwest::StatusCode::CREATED { // 201 Created
                HttpResponse::Ok().json(json!({
                    "status": "success",
                    "message": format!("User {} created successfully.", user_id),
                    "password": password, // Return the password to the user
                }))
            } else {
                let status = response.status();
                let error_body = response.text().await.unwrap_or_else(|_| "Could not read error body".to_string());
                error!("Failed to create Forgejo user '{}'. Status: {}. Body: {}", user_id, status, error_body);
                HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to create user in Forgejo.",
                    "details": error_body,
                }))
            }
        }
        Err(e) => {
            error!("Request to Forgejo API failed: {:?}", e);
            HttpResponse::InternalServerError().json(json!({ "error": "Could not connect to Forgejo service." }))
        }
    }
}


/// Handler to reset a user's password in Forgejo.
/// Corresponds to the `resetUser` Django view.
#[patch("/resetgituser")]
pub async fn reset_forgejo_password(
    session: Session,
    config: web::Data<Config>,
) -> impl Responder {
    // 1. Authentication & Authorization Check
    let user_id = match session.get::<String>("user_id") {
        Ok(Some(id)) if !id.is_empty() => id,
        _ => return HttpResponse::Unauthorized().json(json!({ "error": "Not authenticated" })),
    };
    let permission: i64 = session.get("permissions").unwrap_or_default().unwrap_or(0);
    if permission & PERMISSION_LINUX == 0 {
        return HttpResponse::Forbidden().json(json!({ "error": "Permission denied" }));
    }

    // 2. Prepare request data
    let new_password = generate_password(16);
    let forgejo_payload = json!({
        "password": new_password,
    });

    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/admin/users/{}", config.forge_url, user_id);

    // 3. Send API request to Forgejo
    let res = client
        .patch(&url)
        .bearer_auth(&config.forge_key)
        .json(&forgejo_payload)
        .send()
        .await;

    // 4. Handle response
    match res {
        Ok(response) => {
            if response.status().is_success() { // Typically 200 OK
                HttpResponse::Ok().json(json!({
                    "status": "success",
                    "message": "Password has been reset successfully.",
                    "password": new_password,
                }))
            } else {
                let status = response.status();
                let error_body = response.text().await.unwrap_or_else(|_| "Could not read error body".to_string());
                error!("Failed to reset Forgejo password for '{}'. Status: {}. Body: {}", user_id, status, error_body);
                HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to reset password in Forgejo.",
                    "details": error_body,
                }))
            }
        }
        Err(e) => {
            error!("Request to Forgejo API failed: {:?}", e);
            HttpResponse::InternalServerError().json(json!({ "error": "Could not connect to Forgejo service." }))
        }
    }
}
