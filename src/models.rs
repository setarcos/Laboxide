use serde::{Serialize, Deserialize};
use chrono::{NaiveDate, NaiveDateTime};

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
    pub tea_id: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct SubCourse{
    pub id: i64,
    pub weekday: i64,
    pub room_id: i64,
    pub tea_name: String,
    pub tea_id: String,
    pub year_id: i64,
    pub stu_limit: i64,
    pub course_id: i64,
    pub lag_week: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct SubCourseWithName {
    pub id: i64,
    pub weekday: i64,
    pub room_name: String,
    pub tea_name: String,
    pub tea_id: String,
    pub year_id: i64,
    pub stu_limit: i64,
    pub course_id: i64,
    pub lag_week: i64,
    pub course_name: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Student{
    pub id: i64,
    pub stu_id: String,
    pub stu_name: String,
    pub seat: i64,
    pub subcourse_id: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct CourseSchedule{
    pub id: i64,
    pub week: i64,
    pub name: String,
    pub requirement: String,
    pub course_id: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct CourseFile {
    pub id: i64,
    pub fname: String,
    pub finfo: String,
    pub course_id: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct StudentLog {
    pub id: i64,
    pub stu_id: String,
    pub stu_name: String,
    pub subcourse_id: i64,
    pub room_id: i64,
    pub seat: i64,
    pub lab_name: String,
    pub note: String,
    pub tea_note: String,
    pub tea_name: String,
    pub fin_time: NaiveDateTime,
    pub confirm: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct SubSchedule {
    pub id: i64,
    pub schedule_id: i64,
    pub step: i64,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct StudentTimeline {
    pub id: i64,
    pub stu_id: String,
    pub tea_id: String,
    pub schedule_id: i64,
    pub subschedule: String,
    pub subcourse_id: i64,
    pub note: String, // can be a file path if type == 1
    pub notetype: i64,  // 0 = text, 1 = file
    pub timestamp: NaiveDateTime, // store as ISO string for JSON
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Equipment {
    pub id: i64,
    pub name: String,
    pub serial: String,
    pub value: i64,
    pub position: String,
    pub status: i64,
    pub note: String,
    pub owner_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct EquipmentHistory {
    pub id: i64,
    pub user: String,
    pub borrowed_date: NaiveDateTime,
    pub telephone: String,
    pub note: String,
    pub returned_date: Option<NaiveDateTime>,
    pub item_id: i64,
}
