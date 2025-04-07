// src/main.rs
use actix_web::{App, HttpServer, web, cookie::Key};
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use crate::handler::auth::init_auth_routes;
use crate::handler::user::init_user_routes;
use crate::handler::semester::init_semester_routes;
use crate::handler::course::init_course_routes;
use crate::config::{Config, PERMISSION_ADMIN};
use crate::middleware::CheckPermission;

mod db;
mod models;
mod config;
mod handler;
mod middleware;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize the logger
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    // Initialize the configuration (e.g., secret token)
    let config = Config::from_env();

    // Initialize the database pool
    let db_pool = db::init_db(&config).await.unwrap();

    // Initialize session secret key
    let secret_key = Key::from(&[0; 64]);

    // Start the Actix web server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(db_pool.clone()))
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone(),
            ))
            .configure(init_auth_routes) // Register authentication routes
            .service(
                web::scope("/admin")
                .wrap(CheckPermission::new(PERMISSION_ADMIN))
                .configure(init_user_routes)
                .configure(init_semester_routes)
                .configure(init_course_routes)
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
