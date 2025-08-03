// src/main.rs
use actix_web::{App, HttpServer, web, cookie::Key, middleware::Logger};
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::SameSite;
use base64::{Engine as _, engine::general_purpose};
use crate::handler::auth::init_auth_routes;
use crate::handler::user::init_user_routes;
use crate::handler::semester::{init_semester_routes, get_current_semester};
use crate::handler::course::init_course_adminroutes;
use crate::handler::course::{list_courses, get_course, update_course};
use crate::handler::labroom::{init_labroom_adminroutes, get_labroom, list_labrooms};
use crate::handler::subcourse::{init_subcourse_routes, list_subcourses, list_my_subcourses, get_subcourse};
use crate::handler::group::{init_group_routes, remove_student, list_group, update_student_seat};
use crate::handler::schedule::{init_schedule_routes, list_schedules, get_schedule};
use crate::handler::coursefile::{init_course_file_routes, list_course_files, download_course_file};
use crate::handler::subschedule::{init_subschedule_routes, list_subschedules};
use crate::handler::timeline::{init_timeline_routes, list_timelines_by_schedule};
use crate::handler::equipment::init_equipment_routes;
use crate::handler::meeting::{init_meeting_routes, init_agenda_routes};
use crate::handler::linux::add_linux_user;
use crate::config::{Config, PERMISSION_ADMIN, PERMISSION_TEACHER, PERMISSION_STUDENT, PERMISSION_LAB_MANAGER};
use crate::middleware::CheckPermission;
use handler::studentlog::{init_student_log_routes, default_student_log, confirm_student_log, get_recent_logs, force_student_log, get_student_logs_by_room};
mod db;
mod models;
mod config;
mod handler;
mod middleware;
mod utils;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize the logger
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    let config = Config::from_env();
    // Initialize the database pool
    let db_pool = db::init_db(&config).await.unwrap();

    // Initialize session secret key
    let raw_key = general_purpose::STANDARD.decode(&config.secret)
        .expect("Failed to decode base64 session key. Is it a valid base64 string?");

    let secret_key = Key::from(&raw_key);

    // Start the Actix web server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(db_pool.clone()))
            .wrap(Logger::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                .cookie_same_site(SameSite::Lax) // optional, but recommended for login
                .build(),
            )
            .configure(init_auth_routes) // Register authentication routes
            .service(list_courses)
            .service(list_subcourses)
            .service(get_course)
            .service(get_current_semester)
            .service(get_labroom)
            .service(list_labrooms)
            .service(list_my_subcourses)
            .service(get_subcourse)
            .service(list_schedules)
            .service(get_schedule)
            .service(list_course_files)
            .service(
                web::scope("/admin")
                .wrap(CheckPermission::new(PERMISSION_ADMIN))
                .configure(init_user_routes)
                .configure(init_semester_routes)
                .configure(init_course_adminroutes)
                .configure(init_meeting_routes)
            )
            .service(
                web::scope("/teacher")
                .wrap(CheckPermission::new(PERMISSION_TEACHER | PERMISSION_ADMIN))
                .configure(init_subcourse_routes)
                .configure(init_schedule_routes)
                .configure(init_course_file_routes)
                .configure(init_subschedule_routes)
                .configure(init_equipment_routes)
                .configure(init_agenda_routes)
                .service(update_course)
                .service(remove_student)
                .service(update_student_seat)
                .service(confirm_student_log)
                .service(get_recent_logs)
                .service(list_timelines_by_schedule)
                .service(force_student_log)
            )
            .service(
                web::scope("/lab")
                .wrap(CheckPermission::new(PERMISSION_LAB_MANAGER | PERMISSION_ADMIN))
                .service(get_student_logs_by_room)
                .configure(init_labroom_adminroutes)
            )
            .service(
                web::scope("/stu")
                .wrap(CheckPermission::new(PERMISSION_STUDENT))
                .configure(init_group_routes)
                .configure(init_student_log_routes)
                .service(default_student_log)
                .service(add_linux_user)
            )
            .service(
                web::scope("/member")
                .wrap(CheckPermission::new(PERMISSION_STUDENT | PERMISSION_TEACHER))
                .configure(init_timeline_routes)
                .service(list_group)
                .service(download_course_file)
                .service(list_subschedules)
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
