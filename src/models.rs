use serde::{Serialize, Deserialize};
use chrono::NaiveDate;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub user_id: String,
    pub username: String,
    pub permission: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Semester{
    pub id: i64,
    pub name: String,
    pub start: NaiveDate,
    pub end: NaiveDate,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Course{
    pub id: i64,
    pub name: String,
    pub ename: String,
    pub code: String,
    pub tea_id: String,
    pub tea_name: String,
    pub intro: String,
    pub mailbox: String,
    pub term: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Labroom{
    pub id: i64,
    pub room: String,
    pub name: String,
    pub manager: String,
}
