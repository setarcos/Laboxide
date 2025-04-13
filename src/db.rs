use actix_web::Result;
use sqlx::{Pool, Sqlite, SqlitePool};
use log::error;
use crate::models::{User, Semester, Course, Labroom};
use crate::config::Config;
use crate::models::{SubCourse, SubCourseWithName, StudentGroup, CourseSchedule, CourseFile};
use crate::models::{StudentLog};
use chrono::Utc;

pub async fn init_db(config: &Config) -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePool::connect(&config.database_url).await?;
    Ok(pool)
}

// Db operation for User
pub async fn get_user_by_id(pool: &Pool<Sqlite>, user_id: &str) -> Result<Option<User>, sqlx::Error> {
    let user = sqlx::query_as!(
        User,
        "SELECT user_id, username, permission FROM users WHERE user_id = ?",
        user_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

// Add user
pub async fn add_user(pool: &Pool<Sqlite>, user: User) -> Result<User, sqlx::Error> {
    let rec = sqlx::query_as!(User,
        r#"INSERT INTO users (user_id, username, permission)
        VALUES (?1, ?2, ?3)
        RETURNING user_id, username, permission
        "#,
        user.user_id,
        user.username,
        user.permission
    )
    .fetch_one(pool)
    .await?;

    Ok(rec)
}

// Update user
pub async fn update_user(pool: &Pool<Sqlite>, user: User) -> Result<Option<User>, sqlx::Error> {
    let rec = sqlx::query_as!(User,
        r#"UPDATE users SET username = ?2, permission = ?3
        WHERE user_id = ?1
        RETURNING user_id, username, permission"#,
        user.user_id,
        user.username,
        user.permission
    )
    .fetch_optional(pool)
    .await?;

    Ok(rec)
}

// Delete user
pub async fn delete_user(pool: &Pool<Sqlite>, user_id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM users WHERE user_id = ?",
        user_id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn list_users(pool: &SqlitePool) -> Result<Vec<User>, sqlx::Error> {
    let users = sqlx::query_as!(
        User, "SELECT user_id, username, permission FROM users")
    .fetch_all(pool)
    .await?;

    Ok(users)
}

// Db operation for Semester
pub async fn add_semester(pool: &SqlitePool, semester: Semester) -> Result<Semester, sqlx::Error> {
    let rec = sqlx::query_as!(Semester,
        r#"
        INSERT INTO semesters (name, start, end)
        VALUES (?1, ?2, ?3)
        RETURNING id, name, start, end
        "#,
        semester.name,
        semester.start,
        semester.end
    )
    .fetch_one(pool)
    .await?;

    Ok(rec)
}

pub async fn list_semesters(pool: &SqlitePool) -> Result<Vec<Semester>, sqlx::Error> {
    let semesters = sqlx::query_as!(
        Semester,
        r#"SELECT id, name, start, end FROM semesters"#
    )
    .fetch_all(pool)
    .await?;

    Ok(semesters)
}

pub async fn get_semester_by_id(pool: &SqlitePool, id: i64) -> Result<Option<Semester>, sqlx::Error> {
    let semester = sqlx::query_as!(
        Semester,
        r#"SELECT id, name, start, end FROM semesters WHERE id = ?"#,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(semester)
}

pub async fn get_current_semester(pool: &SqlitePool) -> Result<Option<Semester>, sqlx::Error> {
    let today = chrono::Local::now().naive_local().date();
    let today_str = today.to_string();

    let semester = sqlx::query_as!(
        Semester,
        r#"
        SELECT id, name, start, end
        FROM semesters
        WHERE DATE(start) <= DATE(?1) AND DATE(end) >= DATE(?1)
        "#,
        today_str
    )
    .fetch_optional(pool)
    .await?;

    Ok(semester)
}

pub async fn update_semester(pool: &SqlitePool, id: i64, semester: Semester) -> Result<Option<Semester>, sqlx::Error> {
    let rec = sqlx::query_as!(
        Semester,
        r#"
        UPDATE semesters
        SET name = ?1, start = ?2, end = ?3
        WHERE id = ?4
        RETURNING id, name, start, end
        "#,
        semester.name,
        semester.start,
        semester.end,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(rec)
}

pub async fn delete_semester(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM semesters WHERE id = ?",
        id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn add_course(pool: &SqlitePool, course: Course) -> Result<Course, sqlx::Error> {
    let rec = sqlx::query_as!(
        Course,
        r#"
        INSERT INTO courses (name, ename, code, tea_id, tea_name, intro, mailbox, term)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        RETURNING id, name, ename, code, tea_id, tea_name, intro, mailbox, term
        "#,
        course.name,
        course.ename,
        course.code,
        course.tea_id,
        course.tea_name,
        course.intro,
        course.mailbox,
        course.term
    )
    .fetch_one(pool)
    .await?;

    Ok(rec)
}

pub async fn list_courses(pool: &SqlitePool) -> Result<Vec<Course>, sqlx::Error> {
    let courses = sqlx::query_as!(
        Course,
        r#"SELECT id, name, ename, code, tea_id, tea_name, intro, mailbox, term FROM courses"#
    )
    .fetch_all(pool)
    .await?;

    Ok(courses)
}

pub async fn get_course_by_id(pool: &SqlitePool, id: i64) -> Result<Option<Course>, sqlx::Error> {
    let course = sqlx::query_as!(
        Course,
        r#"SELECT id, name, ename, code, tea_id, tea_name, intro, mailbox, term FROM courses WHERE id = ?"#,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(course)
}

pub async fn update_course(pool: &SqlitePool, id: i64, course: Course) -> Result<Option<Course>, sqlx::Error> {
    let rec = sqlx::query_as!(
        Course,
        r#"
        UPDATE courses
        SET name = ?1, ename = ?2, code = ?3, tea_id = ?4, tea_name = ?5, intro = ?6, mailbox = ?7, term = ?8
        WHERE id = ?9
        RETURNING id, name, ename, code, tea_id, tea_name, intro, mailbox, term
        "#,
        course.name,
        course.ename,
        course.code,
        course.tea_id,
        course.tea_name,
        course.intro,
        course.mailbox,
        course.term,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(rec)
}

pub async fn delete_course(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM courses WHERE id = ?",
        id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

// Db operation for Labroom
pub async fn add_labroom(pool: &SqlitePool, labroom: Labroom) -> Result<Labroom, sqlx::Error> {
    let rec = sqlx::query_as!(
        Labroom,
        r#"
        INSERT INTO labrooms (room, name, manager)
        VALUES (?1, ?2, ?3)
        RETURNING id, room, name, manager
        "#,
        labroom.room,
        labroom.name,
        labroom.manager
    )
    .fetch_one(pool)
    .await?;

    Ok(rec)
}

pub async fn list_labrooms(pool: &SqlitePool) -> Result<Vec<Labroom>, sqlx::Error> {
    let labrooms = sqlx::query_as!(
        Labroom,
        r#"SELECT id, room, name, manager FROM labrooms"#
    )
    .fetch_all(pool)
    .await?;

    Ok(labrooms)
}

pub async fn get_labroom_by_id(pool: &SqlitePool, id: i64) -> Result<Option<Labroom>, sqlx::Error> {
    let labroom = sqlx::query_as!(
        Labroom,
        r#"SELECT id, room, name, manager FROM labrooms WHERE id = ?"#,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(labroom)
}

pub async fn update_labroom(pool: &SqlitePool, id: i64, labroom: Labroom) -> Result<Option<Labroom>, sqlx::Error> {
    let rec = sqlx::query_as!(
        Labroom,
        r#"
        UPDATE labrooms
        SET room = ?1, name = ?2, manager = ?3
        WHERE id = ?4
        RETURNING id, room, name, manager
        "#,
        labroom.room,
        labroom.name,
        labroom.manager,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(rec)
}

pub async fn delete_labroom(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM labrooms WHERE id = ?",
        id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

// Db operations for SubCourse
pub async fn add_subcourse(pool: &SqlitePool, req: SubCourse) -> Result<SubCourse, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        INSERT INTO subcourses (weekday, room_id, tea_name, tea_id, year_id, stu_limit, course_id, lag_week)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        req.weekday,
        req.room_id,
        req.tea_name,
        req.tea_id,
        req.year_id,
        req.stu_limit,
        req.course_id,
        req.lag_week
    )
    .execute(pool)
    .await;

    match result {
        Ok(res) => get_subcourse_by_id(pool, res.last_insert_rowid()).await?.ok_or(sqlx::Error::RowNotFound),
        Err(e) => {
            error!("Failed to add subcourse: {}", e);
            Err(e)
        }
    }
}

pub async fn list_subcourses(
    pool: &SqlitePool,
    course_id: Option<i64>,
    semester_id: Option<i64>,
) -> Result<Vec<SubCourse>, sqlx::Error> {
    if let Some(c_id) = course_id {
        if let Some(s_id) = semester_id {
            // Case 2: Both course_id and semester_id are provided
            sqlx::query_as!(
                SubCourse,
                r#"
                SELECT id, weekday, room_id, tea_name, tea_id, year_id, stu_limit, course_id, lag_week
                FROM subcourses
                WHERE course_id = ?1 AND year_id = ?2
                "#,
                c_id,
                s_id
            )
            .fetch_all(pool)
            .await
        } else {
            // Case 1: Only course_id is provided
            sqlx::query_as!(
                SubCourse,
                r#"
                SELECT id, weekday, room_id, tea_name, tea_id, year_id, stu_limit, course_id, lag_week
                FROM subcourses
                WHERE course_id = ?1
                "#,
                c_id
            )
            .fetch_all(pool)
            .await
        }
    } else {
        // If no course_id is provided, return an empty result
        Ok(vec![]) // Or an error if desired
    }
}

pub async fn get_subcourse_with_name(pool: &SqlitePool, id: i64) -> Result<Option<SubCourseWithName>, sqlx::Error> {
    sqlx::query_as!(
        SubCourseWithName,
        r#"
        SELECT s.id, s.weekday, r.room AS room_name, s.tea_name,
            s.tea_id, s.year_id, s.stu_limit, s.course_id, s.lag_week,
            c.name AS course_name
        FROM subcourses s
            INNER JOIN courses c ON s.course_id = c.id
            INNER JOIN labrooms r ON s.room_id = r.id
            WHERE s.id = ?
        "#,
        id
    )
    .fetch_optional(pool)
    .await
}

pub async fn get_subcourse_by_id(pool: &SqlitePool, id: i64) -> Result<Option<SubCourse>, sqlx::Error> {
    sqlx::query_as!(
        SubCourse,
        r#"
        SELECT
            id, weekday, room_id, tea_name, tea_id, year_id, stu_limit, course_id, lag_week
        FROM subcourses WHERE id = ?
        "#,
        id
    )
    .fetch_optional(pool)
    .await
}

pub async fn update_subcourse(pool: &SqlitePool, id: i64, req: SubCourse) -> Result<Option<SubCourse>, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        UPDATE subcourses
        SET weekday = ?1, room_id = ?2, tea_name = ?3, year_id = ?4,
            stu_limit = ?5, course_id = ?6, lag_week = ?7, tea_id = $8
        WHERE id = ?9
        "#,
        req.weekday,
        req.room_id,
        req.tea_name,
        req.year_id,
        req.stu_limit,
        req.course_id,
        req.lag_week,
        req.tea_id,
        id
    )
    .execute(pool)
    .await;

    match result {
        Ok(res) if res.rows_affected() > 0 => get_subcourse_by_id(pool, id).await,
        Ok(_) => Ok(None),
        Err(e) => {
            error!("Failed to update subcourse {}: {}", id, e);
            Err(e)
        }
    }
}

pub async fn delete_subcourse(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    match sqlx::query!(
        "DELETE FROM subcourses WHERE id = ?",
        id
    )
    .execute(pool)
    .await
    {
        Ok(res) => Ok(res.rows_affected() > 0),
        Err(e) => {
            error!("Failed to delete subcourse {}: {}", id, e);
            Err(e)
        }
    }
}


// Operation for student groups
pub async fn add_student_to_group( pool: &SqlitePool, stu_id: &str,
    stu_name: &str, subcourse_id: i64,) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // Check if the student is already in the group
    let existing: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM student_groups WHERE subcourse_id = ? AND stu_id = ?",
        subcourse_id,
        stu_id
    )
    .fetch_one(&mut *tx)
    .await?;

    if existing > 0 {
        return Ok(()); // Already added
    }
    // Fetch current count
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM student_groups WHERE subcourse_id = ?",
        subcourse_id
    )
    .fetch_one(&mut *tx)
    .await?;

    // Fetch stu_limit
    let stu_limit = sqlx::query_scalar!(
        "SELECT stu_limit FROM subcourses WHERE id = ?",
        subcourse_id
    )
    .fetch_one(&mut *tx)
    .await?;

    if count >= stu_limit {
        return Err(sqlx::Error::RowNotFound); // or a custom error later
    }

    // Insert student with computed seat in one go
    sqlx::query!(
        r#"
        INSERT INTO student_groups (stu_id, stu_name, seat, subcourse_id)
        SELECT ?1, ?2, IFNULL(MAX(seat), 0) + 1, ?3
        FROM student_groups
        WHERE subcourse_id = ?3
        "#,
        stu_id,
        stu_name,
        subcourse_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn remove_student_from_group(
    pool: &SqlitePool,
    stu_id: &str,
    subcourse_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM student_groups WHERE stu_id = ? AND subcourse_id = ?",
        stu_id,
        subcourse_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_student_seat(
    pool: &SqlitePool,
    group_id: i64,
    seat: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE student_groups SET seat = ?1 WHERE id = ?2",
        seat,
        group_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_group_by_subcourse_id(
    pool: &SqlitePool,
    subcourse_id: i64,
) -> Result<Vec<StudentGroup>, sqlx::Error> {
    let rows = sqlx::query_as!(
        StudentGroup,
        "SELECT id, stu_id, stu_name, seat, subcourse_id FROM student_groups WHERE subcourse_id = ? ORDER BY seat",
        subcourse_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn list_student_subcourses(
    pool: &SqlitePool,
    stu_id: &str,
) -> Result<Vec<SubCourseWithName>, sqlx::Error> {
    if let Some(current_semester) = get_current_semester(pool).await? {
        let subcourses = sqlx::query_as!(
            SubCourseWithName,
            r#"
            SELECT
                s.id, s.weekday, r.room AS room_name, s.tea_name, s.tea_id, s.year_id,
                s.stu_limit, s.course_id, s.lag_week, c.name AS course_name
            FROM subcourses s
            JOIN student_groups sg ON sg.subcourse_id = s.id
            JOIN courses c ON s.course_id = c.id
            JOIN labrooms r ON s.room_id = r.id
            WHERE sg.stu_id = ?1 AND s.year_id = ?2
            "#,
            stu_id,
            current_semester.id
        )
        .fetch_all(pool)
        .await?;

        Ok(subcourses)
    } else {
        // No active semester
        Ok(vec![])
    }
}

pub async fn list_teacher_subcourses(
    pool: &SqlitePool,
    tea_id: &str,
) -> Result<Vec<SubCourseWithName>, sqlx::Error> {
    if let Some(current_semester) = get_current_semester(pool).await? {
        let subcourses = sqlx::query_as!(
            SubCourseWithName,
            r#"
            SELECT s.id, s.weekday, r.room AS room_name, s.tea_name,
            s.tea_id, s.year_id, s.stu_limit, s.course_id, s.lag_week,
            c.name AS course_name
            FROM subcourses s
            INNER JOIN courses c ON s.course_id = c.id
            INNER JOIN labrooms r ON s.room_id = r.id
            WHERE s.tea_id = ?1 AND s.year_id = ?2
            "#,
            tea_id,
            current_semester.id
        )
        .fetch_all(pool)
        .await?;

        Ok(subcourses)
    } else {
        // No active semester
        Ok(vec![])
    }
}

// CourseSchedule operations
pub async fn add_schedule(
    pool: &SqlitePool,
    schedule: CourseSchedule,
) -> Result<CourseSchedule, sqlx::Error> {
    let result = sqlx::query_as!(
        CourseSchedule,
        r#"
        INSERT INTO course_schedules (week, name, requirement, course_id)
        VALUES (?1, ?2, ?3, ?4)
        RETURNING id, week, name, requirement, course_id
        "#,
        schedule.week,
        schedule.name,
        schedule.requirement,
        schedule.course_id
    )
    .fetch_one(pool)
    .await?;

    Ok(result)
}

pub async fn list_schedules(pool: &SqlitePool, id: i64) -> Result<Vec<CourseSchedule>, sqlx::Error> {
    let result = sqlx::query_as!(
        CourseSchedule,
        r#"SELECT id, week, name, requirement, course_id
        FROM course_schedules
        WHERE course_id = ?"#,
        id
    )
    .fetch_all(pool)
    .await?;

    Ok(result)
}

pub async fn get_schedule_by_id(
    pool: &SqlitePool,
    id: i64,
) -> Result<Option<CourseSchedule>, sqlx::Error> {
    let result = sqlx::query_as!(
        CourseSchedule,
        r#"SELECT id, week, name, requirement, course_id FROM course_schedules WHERE id = ?"#,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(result)
}

pub async fn update_schedule(
    pool: &SqlitePool,
    id: i64,
    schedule: CourseSchedule,
) -> Result<Option<CourseSchedule>, sqlx::Error> {
    let rec = sqlx::query_as!(
        CourseSchedule,
        r#"
        UPDATE course_schedules
        SET week = ?1, name = ?2, requirement = ?3, course_id = ?4
        WHERE id = ?5
        RETURNING id, week, name, requirement, course_id
        "#,
        schedule.week,
        schedule.name,
        schedule.requirement,
        schedule.course_id,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(rec)
}

pub async fn delete_schedule(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM course_schedules WHERE id = ?",
        id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

// Operations for coursefiles
pub async fn add_course_file(
    pool: &SqlitePool,
    fname: String,
    finfo: String,
    course_id: i64,
) -> Result<CourseFile, sqlx::Error> {
    let rec = sqlx::query_as!(
        CourseFile,
        r#"
        INSERT INTO course_files (fname, finfo, course_id)
        VALUES (?1, ?2, ?3)
        RETURNING id, fname, finfo, course_id
        "#,
        fname,
        finfo,
        course_id
    )
    .fetch_one(pool)
    .await?;

    Ok(rec)
}

pub async fn list_course_files(pool: &SqlitePool, id: i64) -> Result<Vec<CourseFile>, sqlx::Error> {
    let files = sqlx::query_as!(
        CourseFile,
        r#"SELECT id, fname, finfo, course_id FROM course_files
        WHERE course_id = ?"#,
        id
    )
    .fetch_all(pool)
    .await?;

    Ok(files)
}

pub async fn get_course_file_by_id(pool: &SqlitePool, id: i64) -> Result<Option<CourseFile>, sqlx::Error> {
    let file = sqlx::query_as!(
        CourseFile,
        r#"SELECT id, fname, finfo, course_id FROM course_files WHERE id = ?"#,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(file)
}

pub async fn delete_course_file(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"DELETE FROM course_files WHERE id = ?"#,
        id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

// Operations for student_logs

pub async fn add_student_log(pool: &SqlitePool, log: StudentLog) -> Result<StudentLog, sqlx::Error> {
    let now = Utc::now().naive_local();
    let rec = sqlx::query_as!(
        StudentLog,
        r#"
        INSERT INTO student_logs (
            stu_id, stu_name, subcourse_id, room_id, seat,
            lab_name, note, tea_note, tea_name, fin_time, confirm
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0)
        RETURNING *
        "#,
        log.stu_id, log.stu_name, log.subcourse_id, log.room_id, log.seat,
        log.lab_name, log.note, log.tea_note, log.tea_name, now
    )
    .fetch_one(pool)
    .await?;
    Ok(rec)
}

pub async fn update_student_log(pool: &SqlitePool, id: i64, log: StudentLog) -> Result<(), sqlx::Error> {
    let now = Utc::now().naive_local();
    sqlx::query!(
        r#"
        UPDATE student_logs
        SET seat = ?1, note = ?2, fin_time = ?3, lab_name = ?4, fin_time = ?5
        WHERE id = ?6
        "#,
        log.seat, log.note, log.fin_time, log.lab_name, now, id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn confirm_student_log(
    pool: &SqlitePool,
    id: i64,
    tea_note: &String,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().naive_local();
    sqlx::query!(
        "UPDATE student_logs SET tea_note = ?1, confirm = 1, fin_time = ?3 WHERE id = ?2 ",
        tea_note, id, now
    )
    .execute(pool)
    .await?;
    Ok(())
}
