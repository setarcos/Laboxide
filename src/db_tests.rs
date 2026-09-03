    use super::*;
    use crate::models::{
        Course, CourseSchedule, Equipment, EquipmentHistory, Labroom, MeetingAgenda,
        MeetingRoom, Semester, StudentLog, SubCourse,
    };
    use chrono::{Duration as TimeDelta, Local, NaiveDate, NaiveTime};
    use sqlx::sqlite::SqlitePoolOptions;

    /// In-memory SQLite pool (single connection so the DB stays alive) seeded
    /// with the *same* schema file used in production (`init_table.sql`), so
    /// tests never drift from the real table definitions.
    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let schema = include_str!("../init_table.sql");
        for stmt in schema.split(';').map(str::trim).filter(|s| !s.is_empty()) {
            sqlx::query(stmt).execute(&pool).await.unwrap();
        }
        pool
    }

    fn today() -> NaiveDate {
        Local::now().date_naive()
    }

    fn time(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    /// Semester that started exactly 21 days ago -> (today - start).num_weeks()
    /// == 3, so a subcourse with `lag_week = 0` maps to teaching week 4.
    async fn seed_course_base(
        pool: &SqlitePool,
        lag_week: i64,
        stu_limit: i64,
    ) -> (Semester, Course, Labroom, SubCourse) {
        let sem = add_semester(
            pool,
            Semester {
                id: 0,
                name: "2025 秋".into(),
                start: today() - TimeDelta::days(21),
                end: today() + TimeDelta::days(80),
            },
        )
        .await
        .unwrap();
        let course = add_course(
            pool,
            Course {
                id: 0,
                name: "电子实验".into(),
                ename: "EELab".into(),
                code: "EE101".into(),
                tea_id: "T001".into(),
                tea_name: "张老师".into(),
                intro: String::new(),
                mailbox: "zhang@pku.edu.cn".into(),
                term: 1,
            },
        )
        .await
        .unwrap();
        let room = add_labroom(
            pool,
            Labroom {
                id: 0,
                room: "201".into(),
                name: "电子实验室".into(),
                manager: "管理员".into(),
                tea_id: "T001".into(),
            },
        )
        .await
        .unwrap();
        let sub = add_subcourse(
            pool,
            SubCourse {
                id: 0,
                weekday: 2,
                room_id: room.id,
                tea_name: "张老师".into(),
                tea_id: "T001".into(),
                year_id: sem.id,
                stu_limit,
                course_id: course.id,
                lag_week,
            },
        )
        .await
        .unwrap();
        (sem, course, room, sub)
    }

    async fn seed_week_schedules(pool: &SqlitePool, course_id: i64) {
        for week in 3..=4 {
            add_schedule(
                pool,
                CourseSchedule {
                    id: 0,
                    week,
                    name: format!("实验-{}", week),
                    requirement: String::new(),
                    course_id,
                },
            )
            .await
            .unwrap();
        }
    }

    // ---- group / seat logic (transactional) ----

    #[tokio::test]
    async fn group_join_enforces_capacity_and_assigns_seats() {
        let pool = test_pool().await;
        let (_, _, _, sub) = seed_course_base(&pool, 0, 2).await;

        add_student_to_group(&pool, "S1", "学生一", sub.id).await.unwrap();
        add_student_to_group(&pool, "S2", "学生二", sub.id).await.unwrap();

        // over capacity -> rejected
        let err = add_student_to_group(&pool, "S3", "学生三", sub.id)
            .await
            .unwrap_err();
        assert!(matches!(err, sqlx::Error::RowNotFound));

        // duplicate join is idempotent (no extra row)
        add_student_to_group(&pool, "S1", "学生一", sub.id).await.unwrap();

        let group = get_group_by_subcourse_id(&pool, sub.id).await.unwrap();
        assert_eq!(group.len(), 2);
        assert_eq!(group[0].stu_id, "S1");
        assert_eq!(group[0].seat, 1);
        assert_eq!(group[1].stu_id, "S2");
        assert_eq!(group[1].seat, 2);
    }

    // ---- default log / week derivation (recently bug-fixed area) ----

    #[tokio::test]
    async fn default_log_derives_teaching_week_and_lab_name() {
        let pool = test_pool().await;
        let (_, course, room, sub) = seed_course_base(&pool, 0, 40).await;
        seed_week_schedules(&pool, course.id).await;
        add_student_to_group(&pool, "S1", "学生一", sub.id).await.unwrap();

        let log = get_default_log(&pool, "S1", sub.id).await.unwrap();
        // week = (today - semester.start).num_weeks() + 1 - lag_week = 3 + 1 = 4
        assert_eq!(log.lab_name, "实验-4");
        assert_eq!(log.seat, 1);
        assert_eq!(log.room_id, room.id);
        assert_eq!(log.stu_id, "S1");
        assert_eq!(log.confirm, 0);
    }

    #[tokio::test]
    async fn default_log_respects_lag_week_offset() {
        let pool = test_pool().await;
        let (_, course, _, sub) = seed_course_base(&pool, 1, 40).await;
        seed_week_schedules(&pool, course.id).await;
        add_student_to_group(&pool, "S1", "学生一", sub.id).await.unwrap();

        let log = get_default_log(&pool, "S1", sub.id).await.unwrap();
        // lag_week = 1 shifts the class one week later: week = 3 + 1 - 1 = 3
        assert_eq!(log.lab_name, "实验-3");
    }

    #[tokio::test]
    async fn default_log_returns_recent_existing_log_instead_of_fresh_one() {
        let pool = test_pool().await;
        let (_, course, room, sub) = seed_course_base(&pool, 0, 40).await;
        seed_week_schedules(&pool, course.id).await;
        add_student_to_group(&pool, "S1", "学生一", sub.id).await.unwrap();

        let created = add_student_log(
            &pool,
            StudentLog {
                id: 0,
                stu_id: "S1".into(),
                stu_name: "学生一".into(),
                subcourse_id: sub.id,
                room_id: room.id,
                seat: 1,
                lab_name: "实验-4".into(),
                note: "做完了".into(),
                tea_note: String::new(),
                tea_name: String::new(),
                fin_time: Local::now().naive_local(),
                confirm: 0,
            },
        )
        .await
        .unwrap();

        let again = get_default_log(&pool, "S1", sub.id).await.unwrap();
        assert_eq!(again.id, created.id);
        assert_eq!(again.note, "做完了");
    }

    // ---- regression: update_student_log (fixed duplicated fin_time SET) ----

    #[tokio::test]
    async fn student_log_update_stamps_server_time_and_skips_confirmed() {
        let pool = test_pool().await;
        let (_, course, room, sub) = seed_course_base(&pool, 0, 40).await;
        seed_week_schedules(&pool, course.id).await;
        add_student_to_group(&pool, "S1", "学生一", sub.id).await.unwrap();

        let created = add_student_log(
            &pool,
            StudentLog {
                id: 0,
                stu_id: "S1".into(),
                stu_name: "学生一".into(),
                subcourse_id: sub.id,
                room_id: room.id,
                seat: 1,
                lab_name: "实验-4".into(),
                note: "初稿".into(),
                tea_note: String::new(),
                tea_name: String::new(),
                fin_time: Local::now().naive_local(),
                confirm: 0,
            },
        )
        .await
        .unwrap();

        // client sends a stale fin_time; server must stamp `now` instead
        let stale = created.fin_time - TimeDelta::days(1);
        update_student_log(
            &pool,
            created.id,
            StudentLog {
                fin_time: stale,
                seat: 7,
                note: "更新稿".into(),
                lab_name: "实验-4".into(),
                ..created.clone()
            },
        )
        .await
        .unwrap();

        let stored = get_student_log_by_id(&pool, created.id).await.unwrap();
        assert_eq!(stored.seat, 7);
        assert_eq!(stored.note, "更新稿");
        assert_ne!(stored.fin_time, stale, "server must override client fin_time");
        assert!(
            (stored.fin_time - created.fin_time).num_seconds() >= 0,
            "fin_time must be refreshed to server time"
        );

        // once confirmed, further updates are no-ops
        confirm_student_log(&pool, created.id, "完成得不错", "张老师")
            .await
            .unwrap();
        update_student_log(
            &pool,
            created.id,
            StudentLog {
                note: "不应生效".into(),
                ..created
            },
        )
        .await
        .unwrap();
        let stored = get_student_log_by_id(&pool, created.id).await.unwrap();
        assert_eq!(stored.confirm, 1);
        assert_eq!(stored.note, "更新稿");
    }

    // ---- regression: update_equipment_history (fixed `= NULL` -> IS NULL) ----

    #[tokio::test]
    async fn equipment_return_marks_open_record_exactly_once() {
        let pool = test_pool().await;
        let eq = add_equipment(
            &pool,
            Equipment {
                id: 0,
                name: "示波器".into(),
                serial: "DS-1".into(),
                value: 1000,
                position: "A1".into(),
                status: 1,
                note: None,
                owner_id: "T001".into(),
            },
        )
        .await
        .unwrap();
        let now = Local::now().naive_local();
        let history = add_equipment_history(
            &pool,
            EquipmentHistory {
                id: 0,
                user: "借用者".into(),
                borrowed_date: now - TimeDelta::days(1),
                telephone: "13800000000".into(),
                note: String::new(),
                returned_date: None,
                item_id: eq.id,
            },
        )
        .await
        .unwrap();

        let returned = update_equipment_history(&pool, eq.id, now).await.unwrap();
        assert_eq!(returned.id, history.id);
        let rd = returned.returned_date.expect("returned_date must be set");
        assert!((now - rd).num_seconds().abs() <= 2, "returned ~= now");

        // no open record left -> a second return must fail, not silently succeed
        let err = update_equipment_history(&pool, eq.id, now).await.unwrap_err();
        assert!(matches!(err, sqlx::Error::RowNotFound));
    }

    // ---- regression: update_meeting_agenda (fixed confirm=?8 -> ?9) ----

    #[tokio::test]
    async fn meeting_agenda_update_keeps_confirm_from_argument() {
        let pool = test_pool().await;
        let room = add_meeting_room(
            &pool,
            MeetingRoom {
                id: None,
                room: "M101".into(),
                info: String::new(),
            },
        )
        .await
        .unwrap();
        let room_id = room.id.unwrap();
        let d = today();
        let created = add_meeting_agenda(
            &pool,
            MeetingAgenda {
                id: None,
                title: "组会".into(),
                userid: "U1".into(),
                username: "用户一".into(),
                repeat: 0,
                date: d,
                start_time: time(10, 0),
                end_time: time(11, 0),
                room_id,
                confirm: 0,
            },
        )
        .await
        .unwrap();
        let agenda_id = created.id.unwrap();

        // unconfirmed update must stay unconfirmed (bug used to set confirm = room_id)
        let moved = MeetingAgenda {
            id: Some(agenda_id),
            title: "改期组会".into(),
            userid: "U1".into(),
            username: "用户一".into(),
            repeat: 0,
            date: d + TimeDelta::days(1),
            start_time: time(14, 0),
            end_time: time(15, 0),
            room_id,
            confirm: 0,
        };
        let updated = update_meeting_agenda(&pool, agenda_id, moved.clone())
            .await
            .unwrap();
        assert_eq!(updated.confirm, 0, "confirm must not be overwritten by room_id");
        assert_eq!(updated.room_id, room_id);
        assert_eq!(updated.date, d + TimeDelta::days(1));
        assert_eq!(updated.title, "改期组会");

        // an explicit manager confirmation must stick too
        let confirmed = MeetingAgenda { confirm: 1, ..moved };
        let updated = update_meeting_agenda(&pool, agenda_id, confirmed).await.unwrap();
        assert_eq!(updated.confirm, 1);
        assert_eq!(updated.room_id, room_id);
    }

    // ---- meeting conflict detection ----

    #[tokio::test]
    async fn meeting_agenda_conflict_detection() {
        let pool = test_pool().await;
        let room_a = add_meeting_room(
            &pool,
            MeetingRoom { id: None, room: "A".into(), info: String::new() },
        )
        .await
        .unwrap();
        let room_a_id = room_a.id.unwrap();
        let room_b = add_meeting_room(
            &pool,
            MeetingRoom { id: None, room: "B".into(), info: String::new() },
        )
        .await
        .unwrap();
        let room_b_id = room_b.id.unwrap();
        let d = today();

        let base = MeetingAgenda {
            id: None,
            title: "已约".into(),
            userid: "u1".into(),
            username: "用户一".into(),
            repeat: 0,
            date: d,
            start_time: time(10, 0),
            end_time: time(11, 0),
            room_id: room_a_id,
            confirm: 1,
        };
        add_meeting_agenda(&pool, base).await.unwrap();

        let candidate = |title: &str, date: NaiveDate, start: NaiveTime, end: NaiveTime, rid: i64| {
            MeetingAgenda {
                id: None,
                title: title.into(),
                userid: "u2".into(),
                username: "用户二".into(),
                repeat: 0,
                date,
                start_time: start,
                end_time: end,
                room_id: rid,
                confirm: 0,
            }
        };

        // overlapping slot, same day and room -> conflict
        let overlap = candidate("撞车", d, time(10, 30), time(11, 30), room_a_id);
        assert!(check_meeting_conflict(&pool, &overlap).await.unwrap().is_some());

        // touching only at the boundary (end == start) is allowed
        let adjacent = candidate("衔接", d, time(11, 0), time(12, 0), room_a_id);
        assert!(check_meeting_conflict(&pool, &adjacent).await.unwrap().is_none());

        // same time in another room is allowed
        let other_room = candidate("别屋", d, time(10, 30), time(11, 30), room_b_id);
        assert!(check_meeting_conflict(&pool, &other_room).await.unwrap().is_none());

        // weekly-recurring agenda blocks the same weekday in later weeks
        let weekly = add_meeting_agenda(
            &pool,
            MeetingAgenda {
                id: None,
                title: "周例会".into(),
                userid: "u1".into(),
                username: "用户一".into(),
                repeat: 1,
                date: d,
                start_time: time(9, 0),
                end_time: time(10, 0),
                room_id: room_a_id,
                confirm: 0,
            },
        )
        .await
        .unwrap();
        let next_week =
            candidate("下周撞车", d + TimeDelta::days(7), time(9, 30), time(10, 30), room_a_id);
        assert!(check_meeting_conflict(&pool, &next_week).await.unwrap().is_some());

        // a meeting never conflicts with itself
        let self_edit = MeetingAgenda {
            id: weekly.id,
            title: "周例会改".into(),
            userid: "u1".into(),
            username: "用户一".into(),
            repeat: 1,
            date: d,
            start_time: time(9, 0),
            end_time: time(10, 0),
            room_id: room_a_id,
            confirm: 0,
        };
        assert!(check_meeting_conflict(&pool, &self_edit).await.unwrap().is_none());
    }

