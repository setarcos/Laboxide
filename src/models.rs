use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub user_id: String,
    pub username: String,
    pub permission: i64,
}
