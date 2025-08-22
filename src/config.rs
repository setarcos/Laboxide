use dotenv::dotenv;
use std::env;

pub const PERMISSION_ADMIN: i64 = 0b0001;  // Admin: 1st bit
pub const PERMISSION_TEACHER: i64 = 0b0010; // Teacher: 2nd bit
pub const PERMISSION_LAB_MANAGER: i64 = 0b0100; // Lab Manager: 3rd bit
pub const PERMISSION_STUDENT: i64 = 0b1000; // Student: 4th bit
pub const PERMISSION_MEETING_MANAGER: i64 = 0b10000; // Meeting Room Manager: 5th bit
pub const PERMISSION_LINUX : i64 = 0b100000; // Student for Linux course: 6th bit

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub remote_user: String,
    pub remote_host: String,
    pub secret: String,
    pub iaaa_id: String,
    pub iaaa_key: String,
    pub forge_url: String,
    pub forge_key: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv().ok(); // Load the environment variables from the .env file

        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set in .env file");
        let remote_user = env::var("REMOTE_USER").unwrap_or_else(|_| "user".into());
        let remote_host = env::var("REMOTE_HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let secret = env::var("SESSION_SECRET_KEY")
            .expect("SESSION_SECRET_KEY must be set in .env file");
        let iaaa_id = env::var("IAAA_APP_ID")
            .expect("IAAA_APP_ID must be set in .env file");
        let iaaa_key = env::var("IAAA_KEY")
            .expect("IAAA_KEY must be set in .env file");
        let forge_url = env::var("FORGE_URL")
            .expect("FORGE_URL must be set in .env file");
        let forge_key = env::var("FORGE_KEY")
            .expect("FORGE_KEY must be set in .env file");

        Config {
            database_url,
            remote_user,
            remote_host,
            secret,
            iaaa_id,
            iaaa_key,
            forge_url,
            forge_key,
        }
    }
}
