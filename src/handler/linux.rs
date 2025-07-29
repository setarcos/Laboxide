use actix_session::Session;
use actix_web::{post, web, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;
use std::process::Command;
use log::error;
use crate::config::{Config, PERMISSION_LINUX};

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
