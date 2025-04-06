use sqlx::{Pool, Sqlite, SqlitePool};
use crate::models::User;
use crate::config::Config;

pub async fn init_db(config: &Config) -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePool::connect(&config.database_url).await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            user_id TEXT PRIMARY KEY,
            username TEXT NOT NULL,
            permission INTEGER NOT NULL
        )"
    )
    .execute(&pool)
    .await?;
    Ok(pool)
}

pub async fn get_user_by_id(pool: &Pool<Sqlite>, user_id: &str) -> Result<Option<User>, sqlx::Error> {
    let user = sqlx::query_as_unchecked!(
        User,
        "SELECT user_id, username, permission FROM users WHERE user_id = ?",
        user_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

// Add user
pub async fn add_user(pool: &Pool<Sqlite>, user: &User) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO users (user_id, username, permission) VALUES (?1, ?2, ?3)"
    )
    .bind(&user.user_id)
    .bind(&user.username)
    .bind(user.permission)
    .execute(pool)
    .await?;

    Ok(())
}

// Update user
pub async fn update_user(pool: &Pool<Sqlite>, user: &User) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users SET username = ?2, permission = ?3 WHERE user_id = ?1"
    )
    .bind(&user.user_id)
    .bind(&user.username)
    .bind(user.permission)
    .execute(pool)
    .await?;

    Ok(())
}

// Delete user
pub async fn delete_user(pool: &Pool<Sqlite>, user_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn list_users(pool: &SqlitePool) -> Result<Vec<User>, sqlx::Error> {
    let users = sqlx::query_as_unchecked!(
        User, "SELECT user_id, username, permission FROM users")
    .fetch_all(pool)
    .await?;

    Ok(users)
}
