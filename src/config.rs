use dotenv::dotenv;
use std::env;

pub const PERMISSION_ADMIN: i64 = 0b0001;  // Admin: 1st bit
pub const PERMISSION_TEACHER: i64 = 0b0010; // Teacher: 2nd bit
pub const PERMISSION_LAB_MANAGER: i64 = 0b0100; // Lab Manager: 3rd bit
pub const PERMISSION_STUDENT: i64 = 0b1000; // Student: 4th bit

#[derive(Clone)]
pub struct Config {
    pub token_secret: String,
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv().ok(); // Load the environment variables from the .env file

        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set in .env file");
        let token_secret = env::var("TOKEN_SECRET")
            .unwrap_or_else(|_| "mysecret".to_string());

        Config {
            database_url,
            token_secret,
        }
    }
}

