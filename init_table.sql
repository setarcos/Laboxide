CREATE TABLE IF NOT EXISTS users (
    user_id TEXT NOT NULL PRIMARY KEY NOT NULL,
    username TEXT NOT NULL,
    permission INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS semesters (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    start DATE NOT NULL,
    end DATE NOT NULL
);

CREATE TABLE IF NOT EXISTS courses (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(100) NOT NULL,
    ename VARCHAR(200) NOT NULL,
    code VARCHAR(20) NOT NULL,
    tea_id VARCHAR(10) NOT NULL,
    tea_name VARCHAR(50) NOT NULL,
    intro TEXT NOT NULL,
    mailbox VARCHAR(200) NOT NULL,
    term INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS labrooms (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    room VARCHAR(10) NOT NULL,
    name VARCHAR(30) NOT NULL,
    manager VARCHAR(10) NOT NULL,
    tea_id VARCHAR(10) NOT NULL
);

CREATE TABLE IF NOT EXISTS subcourses (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    weekday INTEGER NOT NULL,
    room_id INTEGER NOT NULL REFERENCES labrooms (id),
    tea_name VARCHAR(10) NOT NULL,
    tea_id VARCHAR(10) NOT NULL,
    year_id INTEGER NOT NULL REFERENCES semesters (id),
    stu_limit INTEGER NOT NULL,
    course_id INTEGER NOT NULL REFERENCES courses (id),
    lag_week INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS students (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    stu_id VARCHAR(10) NOT NULL,
    stu_name VARCHAR(10) NOT NULL,
    seat INTEGER NOT NULL,
    subcourse_id INTEGER NOT NULL REFERENCES subcourses (id)
);

CREATE TABLE IF NOT EXISTS course_schedules (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    week INTEGER NOT NULL,
    name VARCHAR(20) NOT NULL,
    requirement VARCHAR(50) NOT NULL,
    course_id INTEGER NOT NULL REFERENCES courses (id)
);

CREATE TABLE IF NOT EXISTS course_files (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    fname VARCHAR(100) NOT NULL,
    finfo VARCHAR(100) NOT NULL,
    course_id INTEGER NOT NULL REFERENCES courses (id)
);

CREATE TABLE IF NOT EXISTS student_logs (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    stu_id VARCHAR(10) NOT NULL,
    stu_name VARCHAR(10) NOT NULL,
    subcourse_id INTEGER NOT NULL REFERENCES subcourses (id),
    room_id INTEGER NOT NULL REFERENCES labrooms (id),
    seat INTEGER NOT NULL,
    lab_name VARCHAR(20) NOT NULL,
    note VARCHAR(50) NOT NULL,
    tea_note VARCHAR(50) NOT NULL,
    tea_name VARCHAR(10) NOT NULL,
    fin_time datetime NOT NULL,
    confirm INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS subschedules (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    schedule_id INTEGER NOT NULL REFERENCES course_schedules (id),
    step INTEGER NOT NULL,
    title VARCHAR(50) NOT NULL
);

CREATE TABLE IF NOT EXISTS student_timelines (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    stu_id VARCHAR(10) NOT NULL,
    tea_id VARCHAR(10) NOT NULL,
    schedule_id INTEGER NOT NULL,
    subschedule VARCHAR(50) NOT NULL,
    subcourse_id INTEGER NOT NULL REFERENCES subcourses (id),
    note VARCHAR(100) NOT NULL,
    notetype INTEGER NOT NULL,
    timestamp datetime NOT NULL
);

CREATE TABLE IF NOT EXISTS equipments (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(100) NOT NULL,
    serial VARCHAR(30) NOT NULL,
    value INTEGER NOT NULL,
    position VARCHAR(20) NOT NULL,
    status INTEGER NOT NULL,
    note VARCHAR(200) NULL,
    owner_id VARCHAR(10) NOT NULL
);

CREATE TABLE IF NOT EXISTS equipment_histories (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    user VARCHAR(20) NOT NULL,
    borrowed_date DATETIME NOT NULL,
    telephone VARCHAR(20) NOT NULL,
    note VARCHAR(200) NOT NULL,
    returned_date DATETIME NULL,
    item_id INTEGER NOT NULL REFERENCES equipments (id)
);

CREATE TABLE IF NOT EXISTS meeting_rooms (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    room VARCHAR(15) NOT NULL,
    info VARCHAR(200) NOT NULL
);

CREATE TABLE IF NOT EXISTS meeting_agendas (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    title VARCHAR(200) NOT NULL,
    userid VARCHAR(12) NOT NULL,
    username VARCHAR(40) NOT NULL,
    repeat integer NOT NULL,
    date date NOT NULL,
    start_time time NOT NULL,
    end_time time NOT NULL,
    room_id integer NOT NULL REFERENCES meeting_rooms (id),
    confirm integer NOT NULL
);
