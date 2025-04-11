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

#[derive(Debug, Serialize)]
pub struct SubCourse{
    pub id: i64,
    pub weekday: i64,
    pub room_id: Labroom,
    pub tea_name: String,
    pub year_id: Semester,
    pub stu_limit: i64,
    pub course_id: Course,
    pub lag_week: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubCourseRequest {
    pub weekday: i64,
    pub room_id: i64,
    pub tea_name: String,
    pub year_id: i64,
    pub stu_limit: i64,
    pub course_id: i64,
    pub lag_week: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SubCourseResponse {
    pub id: i64,
    pub weekday: i64,
    pub room_id: i64,
    pub tea_name: String,
    pub year_id: i64,
    pub stu_limit: i64,
    pub course_id: i64,
    pub lag_week: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StudentGroupResponse {
    pub id: i64,
    pub stu_id: String,
    pub stu_name: String,
    pub seat: i64,
    pub subcourse_id: i64,
}
