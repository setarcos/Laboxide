import sqlite3
from datetime import datetime, timedelta

db_path = "user.db"

# Connect to the database
conn = sqlite3.connect(db_path)
cursor = conn.cursor()

# Calculate the cutoff time (24 hours ago)
cutoff = datetime.now() - timedelta(hours=24)

# Query records newer than cutoff
query = """
SELECT DISTINCT stu_id, subcourse_id, schedule_id
FROM student_timelines
WHERE timestamp >= ?
"""
cursor.execute(query, (cutoff.strftime("%Y-%m-%d %H:%M:%S"),))

# Fetch all student has recent timeline
tlstu = cursor.fetchall()

query = """
SELECT stu_id, subcourse_id, confirm
FROM student_logs
WHERE fin_time >= ?
"""
cursor.execute(query, (cutoff.strftime("%Y-%m-%d %H:%M:%S"),))

# Fetch all student has recent logs
logstu = cursor.fetchall()

logged_pairs = {(stu_id, subcourse_id) for stu_id, subcourse_id, _ in logstu}

missing_logs = [
    (stu_id, subcourse_id, schedule_id)
    for stu_id, subcourse_id, schedule_id in tlstu
    if (stu_id, subcourse_id) not in logged_pairs
]

for row in missing_logs:
    query = "SELECT stu_name, seat FROM students WHERE stu_id = ? AND subcourse_id = ?"
    cursor.execute(query, (row[0], row[1]))
    stu = cursor.fetchone()
    name = stu[0]
    seat = stu[1]
    query = "SELECT name FROM course_schedules WHERE id = ?"
    cursor.execute(query, (row[2],))
    schedule = cursor.fetchone()[0]
    query = "SELECT room_id FROM subcourses WHERE id = ?"
    cursor.execute(query, (row[1],))
    room = cursor.fetchone()[0]
    query = """
        INSERT INTO student_logs (stu_id, stu_name, subcourse_id, room_id, seat,
        lab_name, note, tea_note, tea_name, fin_time, confirm)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """
    cursor.execute(query, (row[0], name, row[1], room, seat, schedule, "", "Log by C", "cronjob",
                           datetime.now().strftime("%Y-%m-%d %H:%M:%S"), 1))
conn.commit()

# Confirm all the unconfirmed logs
query = "UPDATE student_logs SET confirm=1, tea_note=?, tea_name=? WHERE confirm=0 AND fin_time>=?"
cursor.execute(query, ("Conf by C", "cronjob", cutoff.strftime("%Y-%m-%d %H:%M:%S")))

conn.commit()

# Close the connection
conn.close()
