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
    pub cookie_secure: bool,
    pub session_ttl_hours: i64,
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

        // Whether the session cookie should carry the `Secure` attribute.
        // Defaults to true (safe for HTTPS deployments). Set to false when the
        // front end is served over plain HTTP (e.g. local development).
        let cookie_secure = env::var("COOKIE_SECURE")
            .map(|v| {
                let v = v.trim().to_ascii_lowercase();
                v == "1" || v == "true" || v == "yes" || v == "on"
            })
            .unwrap_or(true);

        // Absolute lifetime (in hours) of a session, refreshed on every request
        // (sliding expiration): a user is logged out after this much inactivity.
        let session_ttl_hours = env::var("SESSION_TTL_HOURS")
            .ok()
            .and_then(|v| v.trim().parse::<i64>().ok())
            .filter(|h| *h > 0)
            .unwrap_or(12);

        Config {
            database_url,
            remote_user,
            remote_host,
            secret,
            iaaa_id,
            iaaa_key,
            forge_url,
            forge_key,
            cookie_secure,
            session_ttl_hours,
        }
    }
}
