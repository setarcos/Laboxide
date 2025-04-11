// src/main.rs
use actix_web::{App, HttpServer, web, cookie::Key, middleware::Logger};
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use crate::handler::auth::init_auth_routes;
use crate::handler::user::init_user_routes;
use crate::handler::semester::{init_semester_routes, get_current_semester};
use crate::handler::course::init_course_adminroutes;
use crate::handler::course::{list_courses, get_course, update_course};
use crate::handler::labroom::{init_labroom_adminroutes, get_labroom, list_labrooms};
use crate::handler::subcourse::{init_subcourse_routes, list_subcourses};
use crate::handler::group::{init_group_routes, remove_student};
use crate::config::PERMISSION_LAB_MANAGER;
use crate::config::{Config, PERMISSION_ADMIN, PERMISSION_TEACHER, PERMISSION_STUDENT};
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
            .wrap(Logger::default())
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone(),
            ))
            .configure(init_auth_routes) // Register authentication routes
            .service(list_courses)
            .service(list_subcourses)
            .service(get_course)
            .service(get_current_semester)
            .service(get_labroom)
            .service(list_labrooms)
            .service(
                web::scope("/admin")
                .wrap(CheckPermission::new(PERMISSION_ADMIN))
                .configure(init_user_routes)
                .configure(init_semester_routes)
                .configure(init_course_adminroutes)
            )
            .service(
                web::scope("/stuff")
                .wrap(CheckPermission::new(PERMISSION_TEACHER | PERMISSION_ADMIN))
                .configure(init_subcourse_routes)
                .service(update_course)
                .service(remove_student)
            )
            .service(
                web::scope("/lab")
                .wrap(CheckPermission::new(PERMISSION_LAB_MANAGER | PERMISSION_ADMIN))
                .configure(init_labroom_adminroutes)
            )
            .service(
                web::scope("/stu")
                .wrap(CheckPermission::new(PERMISSION_STUDENT))
                .configure(init_group_routes)
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
