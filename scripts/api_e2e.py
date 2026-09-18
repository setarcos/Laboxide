#!/usr/bin/env python3
"""Laboxide end-to-end API verification script.

What it does
------------
1. Reads ``DATABASE_URL`` from the repo ``.env`` (or ``--database-url``).
2. Backs up the SQLite database (and any ``-wal`` / ``-shm`` siblings).
3. Seeds a few privileged accounts and neutralises "current" test fixtures.
4. Starts the Laboxide binary itself with ``APP_ENV=development`` so that the
   login backdoor (``Student,<id>`` / ``Teacher,<id>``) can be used.
5. Exercises every user-visible HTTP endpoint, including cross-user /
   cross-teacher authorisation checks for data isolation (e.g. one student must
   not be able to read another student's timeline).
6. Stops the server, removes files created under ``uploads/`` and restores the
   database, even when the run is interrupted.

The database and the ``uploads/`` directory are left exactly as they were.

Requirements
------------
``pip install requests`` (the rest is Python standard library).

Usage
-----
    python3 scripts/api_e2e.py                 # full verification
    python3 scripts/api_e2e.py --build         # build the binary first
    python3 scripts/api_e2e.py --keep-db       # keep test data (debugging)
    python3 scripts/api_e2e.py --test-linux    # also call real SSH/Forgejo APIs

Exit code is 0 only when every functional and security check passes; 1 when a
check fails; 2 when the harness itself could not run.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import shutil
import signal
import socket
import sqlite3
import subprocess
import sys
import tempfile
import time
import traceback
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Optional
from urllib.parse import urlparse

try:
    import requests
except ImportError:  # pragma: no cover - environment guard
    sys.stderr.write(
        "This script needs the 'requests' package:\n"
        "    python3 -m pip install requests\n"
    )
    raise SystemExit(2)

REPO_ROOT = Path(__file__).resolve().parents[1]
INIT_SQL = REPO_ROOT / "init_table.sql"

# Permission bits (keep in sync with src/config.rs).
PERM_ADMIN = 0b000001
PERM_TEACHER = 0b000010
PERM_LAB = 0b000100
PERM_STUDENT = 0b001000
PERM_MEETING = 0b010000
PERM_LINUX = 0b100000

# Accounts used by the test run. The backdoor creates/overrides the session.
ADMIN_ID = "e2e_admin"
LAB_ID = "e2e_lab"
MEETING_ID = "e2e_mm"
T1_ID = "t_e2e_1"
T2_ID = "t_e2e_2"
S1_ID = "s_e2e_1"
S2_ID = "s_e2e_2"
S3_ID = "s_e2e_3"
S4_ID = "s_e2e_4"

TOKENS = {
    "admin": f"Teacher,{ADMIN_ID},管理员",
    "t1": f"Teacher,{T1_ID},王老师",
    "t2": f"Teacher,{T2_ID},李老师",
    "lab": f"Teacher,{LAB_ID},实验员",
    "mm": f"Teacher,{MEETING_ID},会议管理员",
    "s1": f"Student,{S1_ID},张三",
    "s2": f"Student,{S2_ID},李四",
    "s3": f"Student,{S3_ID},王五",
    "s4": f"Student,{S4_ID},赵六",
}

EXPECTED_PERMISSIONS = {
    "admin": PERM_ADMIN | PERM_TEACHER,
    "t1": PERM_TEACHER,
    "t2": PERM_TEACHER,
    "lab": PERM_LAB | PERM_TEACHER,
    "mm": PERM_MEETING | PERM_TEACHER,
    "s1": PERM_STUDENT,
    "s2": PERM_STUDENT,
    "s3": PERM_STUDENT,
    "s4": PERM_STUDENT,
}


# --------------------------------------------------------------------------- #
# Tiny test harness
# --------------------------------------------------------------------------- #
@dataclass
class Result:
    name: str
    status: str  # PASS / FAIL / SKIP
    detail: str = ""
    category: str = "functional"


class Suite:
    def __init__(self, base: str, verbose: bool = False, color: bool = True):
        self.base = base
        self.verbose = verbose
        self.color = color
        self.results: list[Result] = []
        self.checks = 0

    def _paint(self, text: str, color: str) -> str:
        if not self.color:
            return text
        return f"{color}{text}\033[0m"

    def record(self, name: str, ok: bool, detail: str = "", category: str = "functional"):
        self.checks += 1
        status = "PASS" if ok else "FAIL"
        self.results.append(Result(name, status, detail, category))
        if ok:
            if self.verbose:
                print(self._paint(f"  PASS  ({category}) {name}", "\033[32m"))
        else:
            print(self._paint(f"  FAIL  ({category}) {name}", "\033[31m"))
            if detail:
                for line in detail.strip().splitlines():
                    print(f"          {line}")

    def skip(self, name: str, detail: str = "", category: str = "functional"):
        self.results.append(Result(name, "SKIP", detail, category))
        if self.verbose:
            print(self._paint(f"  SKIP  ({category}) {name}", "\033[33m"))

    def check(self, name: str, cond: bool, detail: str = "", category: str = "functional"):
        self.record(name, bool(cond), detail, category)
        return cond

    def expect(
        self,
        name: str,
        response: "requests.Response",
        statuses,
        category: str = "functional",
        body_ok: Optional[bool] = None,
        body_detail: str = "",
    ) -> "requests.Response":
        if isinstance(statuses, int):
            statuses = (statuses,)
        statuses = tuple(statuses)
        ok = response.status_code in statuses
        detail = ""
        if not ok:
            detail = f"expected HTTP {list(statuses)}, got {response.status_code}: {_short(response)}"
        elif body_ok is False:
            ok = False
            detail = body_detail or f"unexpected body: {_short(response)}"
        self.record(name, ok, detail, category)
        return response

    def failures(self) -> list[Result]:
        return [r for r in self.results if r.status == "FAIL"]


def _short(response: "requests.Response", limit: int = 300) -> str:
    try:
        text = response.text
    except Exception:  # pragma: no cover
        return "<unreadable body>"
    text = re.sub(r"\s+", " ", text).strip()
    return text[:limit]


def _json_or_none(response: "requests.Response"):
    try:
        return response.json()
    except ValueError:
        return None


class Client:
    def __init__(self, suite: Suite, name: str, timeout: float = 30.0):
        self.suite = suite
        self.name = name
        self.s = requests.Session()
        # Actix closes keep-alive connections after ~5s of inactivity, but
        # urllib3 keeps believing a pooled socket is still usable. Reusing such
        # a half-closed connection makes the request hang until the read
        # timeout expires (the server never even logs it). Tests are short and
        # latency-insensitive, so opt out of connection reuse entirely.
        self.s.headers["Connection"] = "close"
        self.timeout = timeout

    def login(self, token: str):
        return self.s.post(self.suite.base + "/auth", json={"token": token}, timeout=self.timeout)

    def req(self, method: str, path: str, name: str, expect, category="functional", **kw):
        response = self.s.request(
            method, self.suite.base + path, timeout=self.timeout, **kw
        )
        return self.suite.expect(name, response, expect, category)

    def get(self, path, name, expect, category="functional", **kw):
        return self.req("GET", path, name, expect, category, **kw)

    def post(self, path, name, expect, category="functional", **kw):
        return self.req("POST", path, name, expect, category, **kw)

    def put(self, path, name, expect, category="functional", **kw):
        return self.req("PUT", path, name, expect, category, **kw)

    def patch(self, path, name, expect, category="functional", **kw):
        return self.req("PATCH", path, name, expect, category, **kw)

    def delete(self, path, name, expect, category="functional", **kw):
        return self.req("DELETE", path, name, expect, category, **kw)


# --------------------------------------------------------------------------- #
# Database / filesystem helpers
# --------------------------------------------------------------------------- #
def read_env_value(key: str, env_file: Path) -> Optional[str]:
    """Return the raw value of ``key`` from an env file without importing it."""
    if not env_file.exists():
        return None
    pattern = re.compile(rf"^\s*(?:export\s+)?{re.escape(key)}\s*=\s*(.*)\s*$")
    for line in env_file.read_text(encoding="utf-8").splitlines():
        if line.strip().startswith("#"):
            continue
        match = pattern.match(line)
        if match:
            value = match.group(1).strip()
            if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
                value = value[1:-1]
            return value
    return None


def sqlite_path_from_url(database_url: str) -> Optional[Path]:
    """Resolve a sqlx sqlite URL to a filesystem path (relative to CWD)."""
    url = database_url.strip()
    if url.startswith("sqlite://"):
        rest = url[len("sqlite://"):]
    elif url.startswith("sqlite:"):
        rest = url[len("sqlite:"):]
    else:
        raise SystemExit(f"Not a sqlite DATABASE_URL: {database_url!r}")
    rest = rest.split("?", 1)[0]
    if rest in ("", ":memory:"):
        return None
    return Path(rest)


def ensure_schema(db_path: Path):
    con = sqlite3.connect(db_path)
    try:
        have = con.execute(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='users'"
        ).fetchone()
        if not have:
            if not INIT_SQL.exists():
                raise SystemExit(f"Cannot initialize empty database: {INIT_SQL} missing")
            con.executescript(INIT_SQL.read_text(encoding="utf-8"))
            con.commit()
    finally:
        con.close()


def seed_database(db_path: Path):
    """Create the privileged accounts and make the test semester the current one.

    The backdoor only ever grants a fixed permission set, so accounts that need
    ADMIN / LAB_MANAGER / MEETING_MANAGER bits must already exist in ``users``.
    Existing semesters are pushed into the past so that the semester created by
    the tests is unambiguously the "current" one for ``/mycourse`` etc.
    """
    con = sqlite3.connect(db_path)
    try:
        for user_id, username, permission in (
            (ADMIN_ID, "E2E Admin", PERM_ADMIN),
            (LAB_ID, "E2E Lab", PERM_LAB),
            (MEETING_ID, "E2E Meeting", PERM_MEETING),
        ):
            con.execute(
                "INSERT INTO users (user_id, username, permission) VALUES (?, ?, ?) "
                "ON CONFLICT(user_id) DO UPDATE SET username=excluded.username, "
                "permission=excluded.permission",
                (user_id, username, permission),
            )
        con.execute("UPDATE semesters SET end = date('now', '-1 day') WHERE date(end) >= date('now')")
        con.commit()
    finally:
        con.close()


def snapshot_tree(root: Path) -> set[str]:
    if not root.exists():
        return set()
    return {str(p.relative_to(root)) for p in root.rglob("*")}


def cleanup_tree(root: Path, before: set[str], remove_extra_dirs: bool = True) -> list[str]:
    if not root.exists():
        return []
    removed: list[str] = []
    entries = sorted(root.rglob("*"), key=lambda p: len(p.parts), reverse=True)
    for path in entries:
        rel = str(path.relative_to(root))
        if rel in before:
            continue
        if path.is_dir():
            if remove_extra_dirs:
                try:
                    path.rmdir()
                    removed.append(rel + "/")
                except OSError:
                    pass
        else:
            path.unlink()
            removed.append(rel)
    return removed


class DatabaseGuard:
    """Back up the SQLite database and restore it afterwards."""

    def __init__(self, db_path: Optional[Path], keep: bool):
        self.db_path = db_path
        self.keep = keep
        self.tmpdir: Optional[Path] = None
        self.restored = False

    def __enter__(self):
        if self.db_path is None:
            return self
        if not self.db_path.exists():
            raise SystemExit(f"Database file not found: {self.db_path}")
        ensure_schema(self.db_path)
        self.tmpdir = Path(tempfile.mkdtemp(prefix="laboxide-e2e-"))
        for suffix in ("", "-wal", "-shm"):
            src = Path(str(self.db_path) + suffix)
            if src.exists():
                shutil.copy2(src, self.tmpdir / (self.db_path.name + suffix))
        seed_database(self.db_path)
        return self

    def restore(self):
        if self.db_path is None or self.restored:
            return
        self.restored = True
        if self.tmpdir is not None:
            if not self.keep:
                for suffix in ("", "-wal", "-shm"):
                    Path(str(self.db_path) + suffix).unlink(missing_ok=True)
                backup = self.tmpdir / (self.db_path.name)
                if backup.exists():
                    shutil.copy2(backup, self.db_path)
                for suffix in ("-wal", "-shm"):
                    extra = self.tmpdir / (self.db_path.name + suffix)
                    if extra.exists():
                        shutil.copy2(extra, Path(str(self.db_path) + suffix))
            shutil.rmtree(self.tmpdir, ignore_errors=True)

    def __exit__(self, exc_type, exc, tb):
        self.restore()
        return False


class Server:
    """Run the Laboxide binary with the development backdoor enabled."""

    def __init__(self, binary: Path, database_url: str, cwd: Path, tmpdir: Path):
        self.binary = binary
        self.database_url = database_url
        self.cwd = cwd
        self.tmpdir = tmpdir
        self.proc: Optional[subprocess.Popen] = None
        self.log_path = tmpdir / "server.log"

    def start(self, host: str, port: int, timeout: float = 25.0):
        self._assert_port_free(host, port)
        env = os.environ.copy()
        env["APP_ENV"] = "development"
        env["COOKIE_SECURE"] = "false"
        env["DATABASE_URL"] = self.database_url
        env["RUST_LOG"] = env.get("RUST_LOG", "info")
        log = open(self.log_path, "wb")
        try:
            self.proc = subprocess.Popen(
                [str(self.binary)],
                cwd=str(self.cwd),
                env=env,
                stdout=log,
                stderr=subprocess.STDOUT,
            )
        finally:
            log.close()
        deadline = time.time() + timeout
        while time.time() < deadline:
            if self.proc.poll() is not None:
                raise SystemExit(
                    "Server exited during startup:\n" + self.log_tail()
                )
            try:
                requests.get(f"http://{host}:{port}/greet", timeout=2)
                return
            except requests.RequestException:
                time.sleep(0.25)
        raise SystemExit("Server did not become ready in time:\n" + self.log_tail())

    def stop(self):
        if self.proc is None:
            return
        if self.proc.poll() is None:
            self.proc.send_signal(signal.SIGTERM)
            try:
                self.proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait(timeout=5)
        self.proc = None

    def log_tail(self, lines: int = 40) -> str:
        if not self.log_path.exists():
            return "(no server log)"
        content = self.log_path.read_text(errors="replace").splitlines()
        return "\n".join(content[-lines:])

    @staticmethod
    def _assert_port_free(host: str, port: int):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
            sock.settimeout(0.5)
            if sock.connect_ex((host, port)) == 0:
                raise SystemExit(
                    f"Port {port} is already in use. Stop the running Laboxide "
                    f"service (e.g. `systemctl stop Laboxide`) before running this script."
                )


# --------------------------------------------------------------------------- #
# The actual test suite
# --------------------------------------------------------------------------- #
class ApiTest:
    def __init__(self, suite: Suite, args: argparse.Namespace):
        self.suite = suite
        self.args = args
        self.ids: dict[str, Any] = {}
        self.clients: dict[str, Client] = {}
        self.anon = Client(suite, "anon")
        self.today = dt.date.today()
        self.sem_start = self.today - dt.timedelta(weeks=8)
        self.sem_end = self.today + dt.timedelta(weeks=8)
        self.current_week = (self.today - self.sem_start).days // 7 + 1  # lag_week = 0

    # -- helpers ---------------------------------------------------------- #
    def client(self, key: str) -> Client:
        try:
            return self.clients[key]
        except KeyError:  # pragma: no cover - programming error
            raise RuntimeError(f"client {key!r} is not logged in")

    def phase(self, name: str, fn: Callable[[], None]):
        print(f"\n== {name} ==")
        try:
            fn()
        except Exception:
            self.suite.record(f"phase:{name}", False, traceback.format_exc(), "harness")

    def require(self, key: str):
        if key not in self.ids:
            raise RuntimeError(f"missing prerequisite id: {key}")
        return self.ids[key]

    # -- runner ----------------------------------------------------------- #
    def run(self):
        self.phase("auth & permission scopes", self.phase_auth)
        self.phase("admin CRUD", self.phase_admin)
        self.phase("teacher setup (subcourse/schedule/file/equipment)", self.phase_teacher_setup)
        self.phase("students, groups and logs", self.phase_students)
        self.phase("timeline data isolation", self.phase_timeline)
        self.phase("meeting rooms & agendas", self.phase_meeting)
        self.phase("lab manager views", self.phase_lab)
        self.phase("linux / forgejo endpoints (authorization)", self.phase_linux)
        self.phase("global read endpoints & data exposure", self.phase_global_reads)

    # ------------------------------------------------------------------ #
    # Phase 1: auth
    # ------------------------------------------------------------------ #
    def phase_auth(self):
        self.anon.get("/greet", "anonymous /greet is rejected", 401)

        for key, token in TOKENS.items():
            response = self.client_or_create(key).login(token)
            self.suite.expect(f"login backdoor for {key}", response, 200)

        for key, expected in EXPECTED_PERMISSIONS.items():
            client = self.client(key)
            response = client.get("/greet", f"{key} /greet", 200)
            body = _json_or_none(response) or {}
            self.suite.check(
                f"{key} has expected permission mask {expected}",
                body.get("permissions") == expected,
                f"got {body.get('permissions')} in {_short(response)}",
            )

        # Cross-scope denials (functional behaviour of the permission middleware).
        self.anon.get("/admin/user", "anonymous cannot list users", 401)
        self.client("s1").get("/admin/user", "student cannot list users", 403)
        self.client("s1").get("/teacher/equipment", "student cannot list equipment", 403)
        self.client("s1").post(
            "/lab/labroom", "student cannot create labroom", 403,
            json={"id": 0, "room": "X", "name": "X", "manager": "X", "tea_id": "X"},
        )
        self.client("t1").get("/admin/semester", "teacher cannot list semesters", 403)
        self.client("lab").get("/admin/semester", "lab manager cannot list semesters", 403)

        # Logout really ends the session.
        tmp = Client(self.suite, "tmp")
        tmp.login(TOKENS["s1"])
        tmp.get("/logout", "logout succeeds", 200)
        tmp.get("/greet", "session is gone after logout", 401)

    def client_or_create(self, key: str) -> Client:
        if key not in self.clients:
            self.clients[key] = Client(self.suite, key)
        return self.clients[key]

    # ------------------------------------------------------------------ #
    # Phase 2: admin CRUD
    # ------------------------------------------------------------------ #
    def phase_admin(self):
        admin = self.client("admin")

        # Semester -------------------------------------------------------
        semester = admin.post(
            "/admin/semester", "create semester", 200,
            json={
                "id": 0,
                "name": "E2E学期",
                "start": self.sem_start.isoformat(),
                "end": self.sem_end.isoformat(),
            },
        ).json()
        self.ids["semester"] = semester["id"]
        admin.get(f"/admin/semester/{semester['id']}", "get semester", 200)
        listing = admin.get("/admin/semester", "list semesters", 200).json()
        self.suite.check("semester listing contains new semester", any(s["id"] == semester["id"] for s in listing))
        admin.put(
            f"/admin/semester/{semester['id']}", "update semester", 200,
            json={
                "id": semester["id"],
                "name": "E2E学期(改)",
                "start": self.sem_start.isoformat(),
                "end": self.sem_end.isoformat(),
            },
        )
        throwaway = admin.post(
            "/admin/semester", "create throwaway semester", 200,
            json={"id": 0, "name": "临时", "start": "2000-01-01", "end": "2000-02-01"},
        ).json()
        admin.delete(f"/admin/semester/{throwaway['id']}", "delete throwaway semester", 200)
        admin.get(f"/admin/semester/{throwaway['id']}", "deleted semester is gone", 500)

        # Users ----------------------------------------------------------
        created = admin.post(
            "/admin/user", "create user", 201,
            json={"user_id": "e2e_tmp_user", "username": "临时用户", "permission": PERM_STUDENT},
        ).json()
        self.suite.check("created user keeps permission bits", created.get("permission") == PERM_STUDENT, _dumps(created))
        admin.get("/admin/user/e2e_tmp_user", "get user", 200)
        users = admin.get("/admin/user", "list users", 200).json()
        self.suite.check("user listing contains created user", any(u["user_id"] == "e2e_tmp_user" for u in users))
        admin.put(
            "/admin/user", "update user", 200,
            json={"user_id": "e2e_tmp_user", "username": "临时用户2", "permission": PERM_TEACHER},
        )
        admin.delete("/admin/user/e2e_tmp_user", "delete user", 200)
        admin.get("/admin/user/e2e_tmp_user", "deleted user is gone", 500)

        # Courses --------------------------------------------------------
        course_a = self._create_course(admin, "E2E课程A", "Course A", "E2E-A", T1_ID, "王老师")
        course_b = self._create_course(admin, "E2E课程B", "Course B", "E2E-B", T2_ID, "李老师")
        course_linux = self._create_course(admin, "Linux基础", "Linux Basics", "E2E-L", T1_ID, "王老师")
        self.ids["course_a"] = course_a
        self.ids["course_b"] = course_b
        self.ids["course_linux"] = course_linux
        admin.get(f"/course/{course_a}", "get course", 200)
        admin.get("/course", "list courses", 200)
        throwaway_course = self._create_course(admin, "临时课程", "Temp", "E2E-TMP", T1_ID, "王老师")
        renamed = admin.put(
            f"/admin/course/{throwaway_course}", "admin updates course", 200,
            json={"id": throwaway_course, "name": "临时课程(改)", "ename": "Temp2", "code": "E2E-TMP",
                  "tea_id": T1_ID, "tea_name": "王老师", "intro": "x", "mailbox": "x@x", "term": 2},
        ).json()
        self.suite.check("admin course update applies all fields", renamed.get("name") == "临时课程(改)", _dumps(renamed))
        admin.delete(f"/admin/course/{throwaway_course}", "delete throwaway course", 200)

        # Lab rooms ------------------------------------------------------
        labroom = admin.post(
            "/lab/labroom", "create labroom (admin)", 200,
            json={"id": 0, "room": "E2E-R1", "name": "E2E实验室", "manager": LAB_ID, "tea_id": T1_ID},
        ).json()
        self.ids["labroom"] = labroom["id"]
        admin.get(f"/labroom/{labroom['id']}", "get labroom", 200)
        admin.get("/labroom", "list labrooms", 200)
        admin.put(
            f"/lab/labroom/{labroom['id']}", "update labroom", 200,
            json={"id": labroom["id"], "room": "E2E-R1", "name": "E2E实验室(改)", "manager": LAB_ID, "tea_id": T1_ID},
        )

        # Meeting rooms --------------------------------------------------
        room = admin.post(
            "/admin/meeting_room", "create meeting room", 200,
            json={"id": 0, "room": "E2E-M1", "info": "测试会议室"},
        ).json()
        self.ids["meeting_room"] = room["id"]
        admin.get("/teacher/meeting_room", "list meeting rooms", 200)
        admin.put(
            f"/admin/meeting_room/{room['id']}", "update meeting room", 200,
            json={"id": room["id"], "room": "E2E-M1", "info": "测试会议室(改)"},
        )
        throwaway_room = admin.post(
            "/admin/meeting_room", "create throwaway meeting room", 200,
            json={"id": 0, "room": "E2E-MX", "info": "x"},
        ).json()
        admin.delete(f"/admin/meeting_room/{throwaway_room['id']}", "delete throwaway meeting room", 200)

    def _create_course(self, admin: Client, name, ename, code, tea_id, tea_name) -> int:
        response = admin.post(
            "/admin/course", f"create course {code}", 200,
            json={
                "id": 0, "name": name, "ename": ename, "code": code,
                "tea_id": tea_id, "tea_name": tea_name, "intro": "介绍",
                "mailbox": "e2e@pku.edu.cn", "term": 1,
            },
        )
        body = response.json()
        return body["id"]

    # ------------------------------------------------------------------ #
    # Phase 3: teacher setup
    # ------------------------------------------------------------------ #
    def phase_teacher_setup(self):
        t1 = self.client("t1")
        t2 = self.client("t2")
        semester = self.require("semester")
        labroom = self.require("labroom")
        course_a = self.require("course_a")
        course_b = self.require("course_b")
        course_linux = self.require("course_linux")

        sc_a = self._create_subcourse(t1, course_a, semester, labroom, stu_limit=50, name="SC-A")
        sc_cap = self._create_subcourse(t1, course_a, semester, labroom, stu_limit=1, name="SC-CAP")
        sc_linux = self._create_subcourse(t1, course_linux, semester, labroom, stu_limit=50, name="SC-LINUX")
        sc_b = self._create_subcourse(t2, course_b, semester, labroom, stu_limit=50, name="SC-B",
                                       tea_id=T2_ID, tea_name="李老师")
        self.ids.update({"sc_a": sc_a, "sc_cap": sc_cap, "sc_linux": sc_linux, "sc_b": sc_b})

        t1.get(f"/subcourse/{sc_a}", "get subcourse", 200)
        t1.get(f"/subcourse?course_id={course_a}", "list subcourses", 200)

        # Resource-level checks: t2 does not own course A.
        t2.post(
            "/teacher/subcourse", "other teacher cannot create subcourse", 403,
            json={
                "id": 0, "weekday": 2, "room_id": labroom, "tea_name": "李老师",
                "tea_id": T2_ID, "year_id": semester, "stu_limit": 10,
                "course_id": course_a, "lag_week": 0,
            },
        )
        throwaway_sc = self._create_subcourse(t1, course_a, semester, labroom, stu_limit=5, name="SC-TMP")
        t2.put(
            f"/teacher/subcourse/{throwaway_sc}", "other teacher cannot update subcourse", 403,
            json={"id": throwaway_sc, "weekday": 3, "room_id": labroom, "tea_name": "李老师",
                  "tea_id": T2_ID, "year_id": semester, "stu_limit": 9,
                  "course_id": course_a, "lag_week": 0},
        )
        t2.delete(f"/teacher/subcourse/{throwaway_sc}", "other teacher cannot delete subcourse", 403)
        updated_sc = t1.put(
            f"/teacher/subcourse/{throwaway_sc}", "teacher updates own subcourse", 200,
            json={"id": throwaway_sc, "weekday": 3, "room_id": labroom, "tea_name": "王老师",
                  "tea_id": T1_ID, "year_id": semester, "stu_limit": 7,
                  "course_id": course_a, "lag_week": 1},
        ).json()
        self.suite.check("subcourse update changes stu_limit", updated_sc.get("stu_limit") == 7, _dumps(updated_sc))
        t1.delete(f"/teacher/subcourse/{throwaway_sc}", "teacher deletes own subcourse", 200)

        # Schedules ------------------------------------------------------
        self.ids["schedules_a"] = {}
        for week in range(1, 31):
            schedule = t1.post(
                "/teacher/schedule", f"create schedule week {week}", 200,
                json={"id": 0, "week": week, "name": f"实验{week}", "requirement": f"要求{week}", "course_id": course_a},
            ).json()
            self.ids["schedules_a"][week] = schedule
        t1.get(f"/schedule/course/{course_a}", "list schedules", 200)
        self.ids["schedule_cur"] = self.ids["schedules_a"][self.current_week]["id"]
        t1.get(f"/schedule/{self.ids['schedule_cur']}", "get schedule", 200)

        for week in range(1, 31):
            t1.post(
                "/teacher/schedule", f"create linux schedule week {week}", 200,
                json={"id": 0, "week": week, "name": f"Linux实验{week}", "requirement": "r", "course_id": course_linux},
            )

        t2.post(
            "/teacher/schedule", "other teacher cannot create schedule", 403,
            json={"id": 0, "week": 1, "name": "x", "requirement": "x", "course_id": course_a},
        )
        t2.put(
            f"/teacher/schedule/{self.ids['schedule_cur']}", "other teacher cannot update schedule", 403,
            json={"id": self.ids["schedule_cur"], "week": 1, "name": "x", "requirement": "x", "course_id": course_a},
        )
        t2.delete(
            f"/teacher/schedule/{self.ids['schedule_cur']}", "other teacher cannot delete schedule", 403
        )
        throwaway_schedule = t1.post(
            "/teacher/schedule", "create throwaway schedule", 200,
            json={"id": 0, "week": 29, "name": "临时实验", "requirement": "r", "course_id": course_a},
        ).json()["id"]
        t1.put(
            f"/teacher/schedule/{throwaway_schedule}", "teacher updates own schedule", 200,
            json={"id": throwaway_schedule, "week": 29, "name": "临时实验(改)", "requirement": "r2", "course_id": course_a},
        )
        t1.delete(f"/teacher/schedule/{throwaway_schedule}", "teacher deletes own schedule", 200)

        # Sub-schedules --------------------------------------------------
        sub = t1.post(
            "/teacher/subschedule", "create subschedule", 200,
            json={"id": 0, "schedule_id": self.ids["schedule_cur"], "step": 1, "title": "步骤一"},
        ).json()
        self.ids["subschedule"] = sub["id"]
        t1.get(f"/teacher/subschedule/{sub['id']}", "get subschedule", 200)
        t1.put(
            f"/teacher/subschedule/{sub['id']}", "update subschedule (owner)", 200,
            json={"id": sub["id"], "schedule_id": self.ids["schedule_cur"], "step": 1, "title": "步骤一(改)"},
        )
        # Known gap: update_subschedule has no resource-level check.
        t2.put(
            f"/teacher/subschedule/{sub['id']}",
            "SECURITY: other teacher cannot update someone else's subschedule",
            403, category="security",
            json={"id": sub["id"], "schedule_id": self.ids["schedule_cur"], "step": 1, "title": "被篡改"},
        )

        throwaway_sub = t1.post(
            "/teacher/subschedule", "create throwaway subschedule", 200,
            json={"id": 0, "schedule_id": self.ids["schedule_cur"], "step": 2, "title": "临时"},
        ).json()
        t1.delete(f"/teacher/subschedule/{throwaway_sub['id']}", "delete subschedule", 200)

        # Course files ---------------------------------------------------
        upload_dir = self.args.repo_root / "uploads" / "courses" / str(course_a)
        upload_dir.mkdir(parents=True, exist_ok=True)
        payload = b"e2e course file content"
        files = {
            "file": ("e2e_material.txt", payload, "text/plain"),
            "finfo": (None, "测试课件"),
            "course_id": (None, str(course_a)),
        }
        uploaded = t1.post(
            "/teacher/coursefile/upload", "upload course file", 200, files=files
        ).json()
        self.ids["course_file"] = uploaded["id"]
        listing = t1.get(f"/coursefile/list/{course_a}", "list course files", 200).json()
        self.suite.check(
            "course file listing contains the upload",
            any(f["id"] == uploaded["id"] for f in listing),
        )
        downloaded = t1.get(
            f"/member/coursefile/download/{uploaded['id']}", "download course file", 200
        )
        self.suite.check("course file content is intact", downloaded.content == payload, downloaded.text[:200])
        t2.delete(
            f"/teacher/coursefile/{uploaded['id']}",
            "SECURITY: other teacher cannot delete course file",
            403, category="security",
        )
        throwaway_file = t1.post(
            "/teacher/coursefile/upload", "upload throwaway course file", 200,
            files={
                "file": ("e2e_delete_me.txt", b"x", "text/plain"),
                "finfo": (None, "临时"),
                "course_id": (None, str(course_a)),
            },
        ).json()
        t1.delete(f"/teacher/coursefile/{throwaway_file['id']}", "owner deletes course file", 200)
        # Re-uploading the same filename must overwrite, not duplicate.
        again = t1.post(
            "/teacher/coursefile/upload",
            "re-upload same filename overwrites record", 200,
            files={
                "file": ("e2e_material.txt", payload, "text/plain"),
                "finfo": (None, "测试课件v2"),
                "course_id": (None, str(course_a)),
            },
        ).json()
        self.suite.check(
            "re-upload reused the existing record id",
            again.get("id") == uploaded["id"],
            _dumps(again),
        )

        # Equipment ------------------------------------------------------
        equip = t1.post(
            "/teacher/equipment", "create equipment", 200,
            json={"id": 0, "name": "示波器", "serial": "SN-E2E", "value": 1000,
                  "position": "A柜", "status": 1, "note": None, "owner_id": T1_ID},
        ).json()
        self.ids["equipment"] = equip["id"]
        t1.post(
            "/teacher/equipment", "create second equipment (t2)", 200,
            json={"id": 0, "name": "万用表", "serial": "SN-E2E-2", "value": 200,
                  "position": "B柜", "status": 1, "note": None, "owner_id": T2_ID},
        )
        t2_list = t2.get("/teacher/equipment", "t2 lists own equipment", 200).json()
        self.suite.check(
            "equipment listing is scoped to the owner",
            all(e["owner_id"] == T2_ID for e in t2_list) and len(t2_list) == 1,
            _dumps(t2_list),
        )
        t1.get(f"/teacher/equipment/{equip['id']}", "owner reads equipment", 200)
        t2.get(
            f"/teacher/equipment/{equip['id']}",
            "SECURITY: other teacher cannot read equipment by id",
            403, category="security",
        )
        t2.put(
            f"/teacher/equipment/{equip['id']}",
            "SECURITY: other teacher cannot update equipment",
            403, category="security",
            json={"id": equip["id"], "name": "x", "serial": "x", "value": 0,
                  "position": "x", "status": 0, "note": None, "owner_id": T2_ID},
        )
        t2.delete(
            f"/teacher/equipment/{equip['id']}",
            "SECURITY: other teacher cannot delete equipment",
            403, category="security",
        )
        history = t1.post(
            "/teacher/equipment/history", "create equipment history", 200,
            json={"user": "借用者", "telephone": "13800000000", "note": "借用", "item_id": equip["id"]},
        ).json()
        self.ids["equipment_history"] = history["id"]
        t1.get(f"/teacher/equipment/history/{history['id']}", "get equipment history", 200)
        t1.get(f"/teacher/equipment/{equip['id']}/histories", "list equipment histories", 200)
        t1.put(f"/teacher/equipment/history/{equip['id']}", "return equipment", 200)
        t2.get(
            f"/teacher/equipment/{equip['id']}/histories",
            "SECURITY: other teacher cannot list equipment histories",
            403, category="security",
        )
        t1.delete(f"/teacher/equipment/history/{history['id']}", "delete equipment history", 200)
        t1.put(
            f"/teacher/equipment/{equip['id']}", "owner updates equipment", 200,
            json={"id": equip["id"], "name": "示波器(改)", "serial": "SN-E2E", "value": 1200,
                  "position": "A柜", "status": 2, "note": "已校准", "owner_id": T1_ID},
        )
        t1.delete(f"/teacher/equipment/{equip['id']}", "owner deletes equipment", 200)

        # Course update permissions -------------------------------------
        updated = t1.put(
            f"/teacher/course/{course_a}", "teacher updates own course", 200,
            json={"id": course_a, "name": "被改名", "ename": "Hacked", "code": "HACK",
                  "tea_id": T2_ID, "tea_name": "篡改", "intro": "新介绍",
                  "mailbox": "new@pku.edu.cn", "term": 99},
        ).json()
        self.suite.check(
            "teacher update is limited to intro/tea_name/mailbox",
            updated["name"] == "E2E课程A" and updated["tea_id"] == T1_ID and updated["intro"] == "新介绍",
            _dumps(updated),
        )
        t2.put(
            f"/teacher/course/{course_a}", "SECURITY: other teacher cannot update course", 403,
            category="security",
            json={"id": course_a, "name": "x", "ename": "x", "code": "x", "tea_id": T2_ID,
                  "tea_name": "x", "intro": "x", "mailbox": "x", "term": 1},
        )

    def _create_subcourse(self, teacher: Client, course_id: int, semester_id: int,
                          room_id: int, stu_limit: int, name: str,
                          tea_id: str = T1_ID, tea_name: str = "王老师") -> int:
        response = teacher.post(
            "/teacher/subcourse", f"create subcourse {name}", 200,
            json={
                "id": 0, "weekday": 1, "room_id": room_id, "tea_name": tea_name,
                "tea_id": tea_id, "year_id": semester_id, "stu_limit": stu_limit,
                "course_id": course_id, "lag_week": 0,
            },
        )
        return response.json()["id"]

    # ------------------------------------------------------------------ #
    # Phase 4: students, groups, logs
    # ------------------------------------------------------------------ #
    def phase_students(self):
        t1 = self.client("t1")
        t2 = self.client("t2")
        s1, s2, s3, s4 = self.client("s1"), self.client("s2"), self.client("s3"), self.client("s4")
        sc_a = self.require("sc_a")
        sc_cap = self.require("sc_cap")
        sc_linux = self.require("sc_linux")

        s1.post(f"/stu/group/join/{sc_a}", "student 1 joins subcourse", 200)
        s2.post(f"/stu/group/join/{sc_a}", "student 2 joins subcourse", 200)
        s1.post(f"/stu/group/join/{sc_a}", "joining twice is idempotent", 200)
        s1.post(f"/stu/group/join/{sc_cap}", "student 1 joins capacity-1 subcourse", 200)
        s2.post(
            f"/stu/group/join/{sc_cap}",
            "join is rejected when the subcourse is full", 500,
        )
        s2.post(f"/stu/group/join/{sc_linux}", "student 2 joins linux subcourse", 200)

        group = s1.get(f"/member/group/{sc_a}", "member lists group", 200).json()
        self.suite.check("group contains both students", len(group) == 2, _dumps(group))
        s1.get(f"/stu/group/{sc_a}", "student lists group", 200)

        seat_target = next(g for g in group if g["stu_id"] == S1_ID)
        t1.put(f"/teacher/group/seat/{seat_target['id']}/15", "teacher changes seat", 200)
        group_after = s1.get(f"/member/group/{sc_a}", "group after seat change", 200).json()
        self.suite.check(
            "seat change is persisted",
            any(g["stu_id"] == S1_ID and g["seat"] == 15 for g in group_after),
            _dumps(group_after),
        )
        t1.delete(f"/teacher/group/remove/{sc_cap}/{S1_ID}", "teacher removes student", 200)
        s1.post(f"/stu/group/join/{sc_cap}", "student rejoins after removal", 200)
        s1.delete(f"/stu/group/leave/{sc_cap}", "student leaves subcourse", 200)

        # My courses + dynamic Linux permission grant.
        my_t1 = t1.get("/mycourse", "teacher lists own subcourses", 200).json()
        self.suite.check(
            "teacher /mycourse contains the test subcourse",
            any(c["id"] == sc_a for c in my_t1), _dumps(my_t1)[:400],
        )
        my_s1 = s1.get("/mycourse", "student lists own subcourses", 200).json()
        self.suite.check(
            "student /mycourse contains the joined subcourse",
            any(c["id"] == sc_a for c in my_s1), _dumps(my_s1)[:400],
        )
        s2.get("/mycourse", "student 2 lists linux subcourses", 200)
        greet = s2.get("/greet", "greet after /mycourse", 200).json()
        self.suite.check(
            "Linux course grants PERMISSION_LINUX dynamically",
            greet.get("permissions", 0) & PERM_LINUX != 0,
            _dumps(greet),
        )

        # Default log + submission flow.
        default = s1.get(
            f"/stu/student_log/default?subcourse_id={sc_a}&stu_id={S1_ID}",
            "student gets default log", 200,
        ).json()
        expected_lab = f"实验{self.current_week}"
        self.suite.check(
            "default log resolves the current week's experiment",
            default.get("lab_name") == expected_lab,
            f"expected {expected_lab!r}, got {default.get('lab_name')!r}",
        )
        self.ids["default_log"] = default

        payload = dict(default)
        payload["confirm"] = 0
        payload["note"] = "e2e note"
        payload["fin_time"] = dt.datetime.now().strftime("%Y-%m-%dT%H:%M:%S")
        created = s1.post("/stu/student_log", "student submits log", 200, json=payload).json()
        self.ids["log_s1"] = created["id"]
        self.suite.check("submitted log is not confirmed", created.get("confirm") == 0, _dumps(created))
        s1.post("/stu/student_log", "duplicate log within 5h is rejected", 500, json=payload)
        update = dict(payload)
        update["note"] = "e2e note updated"
        s1.put(f"/stu/student_log/{created['id']}", "student updates own log", 200, json=update)
        s2.put(
            f"/stu/student_log/{created['id']}",
            "SECURITY: another student cannot update the log", 403, category="security",
            json={**update, "stu_id": S1_ID},
        )
        s1.post(
            "/stu/student_log",
            "SECURITY: student cannot submit a log under another id", 403, category="security",
            json={**payload, "stu_id": S2_ID},
        )

        recent = t1.get(f"/teacher/student_log/recent/{sc_a}", "teacher lists recent logs", 200).json()
        self.suite.check("recent logs contain the submission", any(l["id"] == created["id"] for l in recent), _dumps(recent)[:400])
        s1.put(
            f"/teacher/student_log/confirm/{created['id']}",
            "SECURITY: student cannot confirm a log", 403, category="security",
            json={"tea_note": "x"},
        )
        t1.put(
            f"/teacher/student_log/confirm/{created['id']}", "teacher confirms log", 200,
            json={"tea_note": "已检查"},
        )
        recent = t1.get(f"/teacher/student_log/recent/{sc_a}", "recent logs after confirm", 200).json()
        confirmed = next((l for l in recent if l["id"] == created["id"]), {})
        self.suite.check(
            "confirmation stores tea_note and confirm=1",
            confirmed.get("confirm") == 1 and confirmed.get("tea_note") == "已检查",
            _dumps(confirmed),
        )

        # A log submitted by s2 and confirmed by a non-owning teacher.
        s2_payload = dict(default)
        s2_payload.update({"stu_id": S2_ID, "stu_name": "李四", "confirm": 0,
                           "note": "s2", "fin_time": dt.datetime.now().strftime("%Y-%m-%dT%H:%M:%S")})
        s2_log = s2.post("/stu/student_log", "student 2 submits log", 200, json=s2_payload).json()
        t2.put(
            f"/teacher/student_log/confirm/{s2_log['id']}",
            "SECURITY: unrelated teacher cannot confirm another class's log",
            403, category="security",
            json={"tea_note": "越权"},
        )

        # force_student_log for a student who never submitted.
        s3.post(f"/stu/group/join/{sc_a}", "student 3 joins subcourse", 200)
        s4.post(f"/stu/group/join/{sc_a}", "student 4 joins subcourse", 200)
        forced = t1.put(
            f"/teacher/student_log/force/{sc_a}/{S3_ID}",
            "teacher force-creates a log", 200,
        ).json()
        self.suite.check(
            "forced log is confirmed and teacher-signed",
            forced.get("confirm") == 1 and forced.get("tea_name") != "",
            _dumps(forced),
        )

        # Known gaps: cross-class reads/writes on the log endpoints.
        t2.get(
            f"/teacher/student_log/recent/{sc_a}",
            "SECURITY: unrelated teacher cannot list another class's recent logs",
            403, category="security",
        )
        t2.put(
            f"/teacher/student_log/force/{sc_a}/{S4_ID}",
            "SECURITY: unrelated teacher cannot force-create a log for another class",
            403, category="security",
        )

        # Known gap: any student can query another student's default log.
        s1.get(
            f"/stu/student_log/default?subcourse_id={sc_a}&stu_id={S3_ID}",
            "SECURITY: student cannot read another student's default log",
            403, category="security",
        )

        # Lab manager views -------------------------------------------------
        today_str = self.today.isoformat()
        self.client("lab").post(
            f"/lab/student_log/room/{self.require('labroom')}",
            "lab manager queries room logs", 200,
            json={"start_time": f"{today_str}T00:00:00", "end_time": f"{today_str}T23:59:59"},
        )
        s1.post(
            f"/lab/student_log/room/{self.require('labroom')}",
            "SECURITY: student cannot query room logs", 403, category="security",
            json={"start_time": f"{today_str}T00:00:00", "end_time": f"{today_str}T23:59:59"},
        )

    # ------------------------------------------------------------------ #
    # Phase 5: timeline isolation
    # ------------------------------------------------------------------ #
    def phase_timeline(self):
        s1, s2, s3 = self.client("s1"), self.client("s2"), self.client("s3")
        t1, t2 = self.client("t1"), self.client("t2")
        sc_a = self.require("sc_a")
        schedule_id = self.require("schedule_cur")

        def timeline_fields(notetype: int, schedule: Optional[int] = None, tea_id: str = "-"):
            return {
                "stu_id": S1_ID,
                "tea_id": tea_id,
                "schedule_id": str(schedule if schedule is not None else schedule_id),
                "subschedule": "1",
                "subcourse_id": str(sc_a),
                "notetype": str(notetype),
            }

        text_resp = s1.post(
            "/member/timeline", "create text timeline", 200,
            data=timeline_fields(0), files={"note": (None, "文本时间线")},
        )
        text_id = text_resp.json()["id"]
        self.ids["timeline_text"] = text_id

        file_content = b"timeline attachment"
        file_resp = s1.post(
            "/member/timeline", "create file timeline", 200,
            data=timeline_fields(1),
            files={"file": ("e2e_timeline.txt", file_content, "text/plain")},
        )
        file_id = file_resp.json()["id"]
        self.ids["timeline_file"] = file_id

        own = s1.get(f"/member/timeline/student/{sc_a}/{S1_ID}", "student reads own timeline", 200).json()
        self.suite.check(
            "student's own timeline contains both entries",
            {text_id, file_id} <= {e["id"] for e in own}, _dumps(own)[:400],
        )

        self.anon.get(
            f"/member/timeline/student/{sc_a}/{S1_ID}", "anonymous cannot read timeline", 401
        )
        s2.get(
            f"/member/timeline/student/{sc_a}/{S1_ID}",
            "SECURITY: another student cannot read someone else's timeline",
            (401, 403), category="security",
        )
        t2_list = t2.get(
            f"/member/timeline/student/{sc_a}/{S1_ID}",
            "unrelated teacher sees no foreign timeline entries", 200,
        ).json()
        self.suite.check(
            "unrelated teacher's timeline query returns nothing",
            t2_list == [], _dumps(t2_list)[:400],
        )
        t1_list = t1.get(
            f"/member/timeline/student/{sc_a}/{S1_ID}", "owning teacher reads student timeline", 200
        ).json()
        self.suite.check(
            "owning teacher sees the timeline",
            {text_id, file_id} <= {e["id"] for e in t1_list}, _dumps(t1_list)[:400],
        )

        # A teacher-authored checkpoint is recorded with their own id as tea_id,
        # unlike student-authored entries which use "-".
        teacher_entry = t1.post(
            "/member/timeline", "teacher adds a checkpoint", 200,
            data=timeline_fields(0, tea_id=T1_ID), files={"note": (None, "教师检查点")},
        ).json()
        t1_list = t1.get(
            f"/member/timeline/student/{sc_a}/{S1_ID}", "owning teacher reads student timeline (2)", 200
        ).json()
        self.suite.check(
            "owning teacher also sees the teacher-authored entry",
            teacher_entry["id"] in {e["id"] for e in t1_list}, _dumps(t1_list)[:400],
        )
        t1.get(
            f"/teacher/timeline/schedule/{sc_a}/{schedule_id}",
            "teacher lists class timeline by schedule", 200,
        )
        # Known gap: no course-level check on the by-schedule listing.
        t2.get(
            f"/teacher/timeline/schedule/{sc_a}/{schedule_id}",
            "SECURITY: unrelated teacher cannot list another class's timeline by schedule",
            403, category="security",
        )

        download = s1.get(f"/member/timeline/file/{file_id}", "owner downloads timeline file", 200)
        self.suite.check("timeline file content is intact", download.content == file_content, download.text[:200])
        t1.get(f"/member/timeline/file/{file_id}", "owning teacher downloads timeline file", 200)
        s2.get(
            f"/member/timeline/file/{file_id}",
            "SECURITY: another student cannot download someone else's timeline file",
            403, category="security",
        )
        s1.get(
            f"/member/timeline/file/{text_id}",
            "text timeline has no downloadable file", 400,
        )

        s2.delete(
            f"/member/timeline/{file_id}",
            "SECURITY: another student cannot delete someone else's timeline",
            403, category="security",
        )
        s3.post(
            "/member/timeline",
            "SECURITY: student cannot create a timeline under another id", 403, category="security",
            data={**timeline_fields(0), "stu_id": S2_ID}, files={"note": (None, "x")},
        )

        # Deleting before the related log is confirmed is allowed. Use a week
        # whose experiment has no confirmed log for this student.
        other_schedule = self.ids["schedules_a"][1]["id"]
        deletable = s1.post(
            "/member/timeline", "create timeline on an unconfirmed schedule", 200,
            data=timeline_fields(0, other_schedule), files={"note": (None, "可删除")},
        ).json()
        s1.delete(
            f"/member/timeline/{deletable['id']}",
            "owner deletes own timeline before confirmation", 200,
        )

        # Once the student's log for the schedule is confirmed, deletion is blocked.
        blocked = s1.post(
            "/member/timeline", "create timeline for confirmed schedule", 200,
            data=timeline_fields(0), files={"note": (None, "受保护")},
        ).json()
        s1.delete(
            f"/member/timeline/{blocked['id']}",
            "student cannot delete timeline after teacher confirmation", 403,
        )
        t1.delete(
            f"/member/timeline/{teacher_entry['id']}",
            "recording teacher deletes own timeline entry", 200,
        )

    # ------------------------------------------------------------------ #
    # Phase 6: meetings
    # ------------------------------------------------------------------ #
    def phase_meeting(self):
        t1, t2, mm = self.client("t1"), self.client("t2"), self.client("mm")
        room = self.require("meeting_room")
        date = (self.today + dt.timedelta(days=7)).isoformat()

        def agenda(title, start, end, userid=T1_ID, username="王老师"):
            return {
                "id": 0, "title": title, "userid": userid, "username": username,
                "repeat": 0, "date": date, "start_time": start, "end_time": end,
                "room_id": room, "confirm": 0,
            }

        ag1 = t1.post(
            "/teacher/meeting_agenda", "create agenda", 200,
            json=agenda("组会", "10:00:00", "11:00:00"),
        ).json()
        self.ids["agenda1"] = ag1["id"]
        self.suite.check("non-manager agenda needs confirmation", ag1.get("confirm") == 0, _dumps(ag1))
        t1.post(
            "/teacher/meeting_agenda", "SECURITY: overlapping agenda is rejected", 409,
            category="functional", json=agenda("冲突组会", "10:30:00", "11:30:00"),
        )
        ag2 = t1.post(
            "/teacher/meeting_agenda", "create non-overlapping agenda", 200,
            json=agenda("下午组会", "12:00:00", "13:00:00"),
        ).json()
        self.ids["agenda2"] = ag2["id"]

        t1.get(f"/teacher/meeting_agenda/{ag1['id']}", "get agenda", 200)
        t1.get(f"/teacher/meeting_agenda/room/{room}", "list agendas by room", 200)
        t2.put(
            f"/teacher/meeting_agenda/{ag1['id']}", "SECURITY: other user cannot update agenda", 403,
            category="security", json={**agenda("篡改", "10:00:00", "11:00:00"), "id": ag1["id"]},
        )
        t1.put(
            f"/teacher/meeting_agenda/{ag1['id']}", "creator updates unconfirmed agenda", 200,
            json={**agenda("组会(改)", "10:00:00", "11:00:00"), "id": ag1["id"]},
        )
        t1.put(
            f"/teacher/meeting_agenda/{ag1['id']}/confirm",
            "non-manager cannot confirm agenda", 403,
            json={},
        )
        mm.put(f"/teacher/meeting_agenda/{ag1['id']}/confirm", "meeting manager confirms agenda", 200)
        t1.put(
            f"/teacher/meeting_agenda/{ag1['id']}", "confirmed agenda can no longer be changed", 403,
            json={**agenda("再改", "10:00:00", "11:00:00"), "id": ag1["id"]},
        )
        auto = mm.post(
            "/teacher/meeting_agenda", "manager agenda is auto-confirmed", 200,
            json={**agenda("管理员会议", "14:00:00", "15:00:00", MEETING_ID, "会议管理员"), "confirm": 0},
        ).json()
        self.suite.check("meeting manager agenda auto-confirmed", auto.get("confirm") == 1, _dumps(auto))
        mm.delete(f"/teacher/meeting_agenda/{auto['id']}", "manager deletes own agenda", 200)

        # Deleting requires the same check as updating: the creator may delete
        # only while the agenda is still unconfirmed, everyone else is refused.
        t2.delete(
            f"/teacher/meeting_agenda/{ag2['id']}",
            "SECURITY: unrelated teacher cannot delete someone else's agenda",
            403, category="security",
        )
        t1.delete(f"/teacher/meeting_agenda/{ag2['id']}", "creator deletes own unconfirmed agenda", 200)
        # ag1 has been confirmed by the meeting manager, so even its creator
        # must no longer be able to delete it.
        t1.delete(
            f"/teacher/meeting_agenda/{ag1['id']}",
            "SECURITY: confirmed agenda cannot be deleted by its creator",
            403, category="security",
        )
        mm.delete(f"/teacher/meeting_agenda/{ag1['id']}", "meeting manager deletes confirmed agenda", 200)

    # ------------------------------------------------------------------ #
    # Phase 7: lab manager CRUD
    # ------------------------------------------------------------------ #
    def phase_lab(self):
        lab, s1 = self.client("lab"), self.client("s1")
        created = lab.post(
            "/lab/labroom", "lab manager creates labroom", 200,
            json={"id": 0, "room": "E2E-R2", "name": "临时实验室", "manager": LAB_ID, "tea_id": T1_ID},
        ).json()
        lab.put(
            f"/lab/labroom/{created['id']}", "lab manager updates labroom", 200,
            json={"id": created["id"], "room": "E2E-R2", "name": "临时实验室(改)", "manager": LAB_ID, "tea_id": T1_ID},
        )
        lab.delete(f"/lab/labroom/{created['id']}", "lab manager deletes labroom", 200)
        s1.delete(
            f"/lab/labroom/{created['id']}",
            "SECURITY: student cannot delete labroom", 403, category="security",
        )

    # ------------------------------------------------------------------ #
    # Phase 8: linux / forgejo authorization
    # ------------------------------------------------------------------ #
    def phase_linux(self):
        s1, s2 = self.client("s1"), self.client("s2")
        s1.post("/stu/adduser", "student without Linux bit cannot add linux user", 403,
                json={"sshkey": "ssh-ed25519 AAAA e2e"})
        s1.get("/stu/showdiff", "student without Linux bit cannot run showdiff", 403)
        s1.post("/stu/copyvihw", "student without Linux bit cannot copy vim homework", 403)
        s1.post("/stu/gituser", "student without Linux bit cannot create git user", 403)
        s1.patch("/stu/resetgituser", "student without Linux bit cannot reset git password", 403)

        # Anonymous requests stay 401 (authentication), even for endpoints whose
        # authorization failure is 403.
        self.anon.post("/stu/adduser", "anonymous adduser is 401", 401,
                       json={"sshkey": "ssh-ed25519 AAAA e2e"})
        self.anon.get("/stu/showdiff", "anonymous showdiff is 401", 401)
        self.anon.put(f"/teacher/equipment/{self.require('equipment')}",
                      "anonymous equipment update is 401", 401,
                      json={"id": 0, "name": "x", "serial": "x", "value": 0,
                            "position": "x", "status": 0, "note": None, "owner_id": "x"})
        self.anon.delete("/teacher/meeting_agenda/999999",
                         "anonymous agenda delete is 401", 401)

        if not self.args.test_linux:
            self.suite.skip(
                "linux/forgejo happy paths (needs --test-linux; would touch the real host)", "",
            )
            return

        # Opt-in: these really call the configured SSH host / Forgejo instance.
        for name, method, path, kwargs in (
            ("adduser", "post", "/stu/adduser", {"json": {"sshkey": "ssh-ed25519 AAAA e2e"}}),
            ("copyvihw", "post", "/stu/copyvihw", {}),
            ("showdiff", "get", "/stu/showdiff", {}),
        ):
            response = getattr(s2, method)(path, f"linux: {name}", (200, 500), **kwargs)
            self.suite.check(
                f"linux {name} reached the backend (status {response.status_code})",
                response.status_code in (200, 500),
                _short(response),
                category="informational",
            )

    # ------------------------------------------------------------------ #
    # Phase 9: global reads + field exposure
    # ------------------------------------------------------------------ #
    def phase_global_reads(self):
        course_a = self.require("course_a")
        self.anon.get("/semester/current", "anonymous reads current semester", 200)
        self.anon.get("/course", "anonymous lists courses", 200)
        self.anon.get(f"/course/{course_a}", "anonymous gets course", 200)
        self.anon.get("/labroom", "anonymous lists labrooms", 200)
        self.anon.get(f"/labroom/{self.require('labroom')}", "anonymous gets labroom", 200)
        self.anon.get(f"/subcourse?course_id={course_a}", "anonymous lists subcourses", 200)
        self.anon.get(f"/subcourse/{self.require('sc_a')}", "anonymous gets subcourse", 200)
        self.anon.get("/mycourse", "anonymous has no mycourse", 401)
        self.anon.get(f"/coursefile/list/{course_a}", "anonymous lists course files", 200)
        self.anon.get(f"/member/subschedules/{self.require('schedule_cur')}",
                      "anonymous cannot list subschedules", 401)
        self.client("s1").get(f"/member/subschedules/{self.require('schedule_cur')}",
                              "member lists subschedules", 200)

        # tea_id must be hidden from non-teachers.
        anon_courses = self.anon.get("/course", "anon course list (for tea_id check)", 200).json()
        self.suite.check(
            "course list hides tea_id from anonymous users",
            all(c["tea_id"] == "" for c in anon_courses), _dumps(anon_courses)[:400],
        )
        t1_courses = self.client("t1").get("/course", "teacher course list (for tea_id check)", 200).json()
        self.suite.check(
            "course list exposes tea_id to teachers",
            any(c["tea_id"] != "" for c in t1_courses), _dumps(t1_courses)[:400],
        )
        anon_rooms = self.anon.get("/labroom", "anon labroom list (for tea_id check)", 200).json()
        self.suite.check(
            "labroom list hides tea_id from anonymous users",
            all(r["tea_id"] == "" for r in anon_rooms), _dumps(anon_rooms)[:400],
        )
        anon_subs = self.anon.get(f"/subcourse?course_id={course_a}", "anon subcourse list (for tea_id check)", 200).json()
        self.suite.check(
            "subcourse list hides tea_id from anonymous users",
            all(s["tea_id"] == "" for s in anon_subs), _dumps(anon_subs)[:400],
        )


def _dumps(value: Any) -> str:
    try:
        return json.dumps(value, ensure_ascii=False)[:600]
    except (TypeError, ValueError):
        return str(value)[:600]


# --------------------------------------------------------------------------- #
# Entry point
# --------------------------------------------------------------------------- #
def newest_source_mtime() -> float:
    newest = 0.0
    patterns = ["Cargo.toml", "Cargo.lock", "src/**/*.rs"]
    for pattern in patterns:
        for path in REPO_ROOT.glob(pattern) if "**" in pattern else [REPO_ROOT / pattern]:
            if path.exists():
                newest = max(newest, path.stat().st_mtime)
    return newest


def pick_binary(explicit: Optional[str]) -> Path:
    if explicit:
        path = Path(explicit)
        if not path.exists():
            raise SystemExit(f"Binary not found: {path}")
        return path
    candidates = [p for p in (REPO_ROOT / "target/release/Laboxide",
                              REPO_ROOT / "target/debug/Laboxide") if p.exists()]
    if not candidates:
        raise SystemExit("No Laboxide binary found; build it first (cargo build) or pass --build")
    return max(candidates, key=lambda p: p.stat().st_mtime)


def build_binary(release: bool = True) -> Path:
    print("Building Laboxide ...")
    env = os.environ.copy()
    env.setdefault("DATABASE_URL", "sqlite://./example.db")
    cmd = ["cargo", "build"] + (["--release"] if release else [])
    subprocess.run(cmd, cwd=str(REPO_ROOT), env=env, check=True)
    return pick_binary(None)


def parse_args(argv=None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--base-url", default="http://127.0.0.1:8080",
                        help="base URL of the API (default: %(default)s)")
    parser.add_argument("--database-url", default=None,
                        help="override DATABASE_URL (default: read from .env)")
    parser.add_argument("--binary", default=None, help="path to the Laboxide binary")
    parser.add_argument("--build", action="store_true", help="run cargo build --release first")
    parser.add_argument("--keep-db", action="store_true", help="do not restore the database afterwards")
    parser.add_argument("--keep-uploads", action="store_true", help="do not clean files created under uploads/")
    parser.add_argument("--test-linux", action="store_true",
                        help="also exercise the real SSH/Forgejo endpoints (touches external systems)")
    parser.add_argument("--verbose", action="store_true", help="print passing checks too")
    parser.add_argument("--no-color", action="store_true")
    parser.add_argument("--start-timeout", type=float, default=25.0)
    return parser.parse_args(argv)


def main(argv=None) -> int:
    args = parse_args(argv)
    args.repo_root = REPO_ROOT

    database_url = args.database_url or read_env_value("DATABASE_URL", REPO_ROOT / ".env")
    if not database_url:
        print("DATABASE_URL not found in .env; pass --database-url", file=sys.stderr)
        return 2
    db_path = sqlite_path_from_url(database_url)
    if db_path is not None and not db_path.is_absolute():
        db_path = (REPO_ROOT / db_path).resolve()

    base = args.base_url.rstrip("/")
    parsed = urlparse(base)
    host = parsed.hostname or "127.0.0.1"
    port = parsed.port or 80
    if port != 8080:
        print(f"warning: the server always binds 127.0.0.1:8080, but --base-url uses port {port}",
              file=sys.stderr)

    if args.build:
        binary = build_binary(release=True)
    else:
        binary = pick_binary(args.binary)
    newest_source = newest_source_mtime()
    if binary.stat().st_mtime < newest_source and not args.build:
        print(
            f"warning: {binary} is older than the newest source file; it may be stale. "
            f"Consider running with --build.",
            file=sys.stderr,
        )

    color = sys.stdout.isatty() and not args.no_color
    suite = Suite(base, verbose=args.verbose, color=color)
    tmpdir = Path(tempfile.mkdtemp(prefix="laboxide-e2e-"))

    uploads = REPO_ROOT / "uploads"
    uploads_before = snapshot_tree(uploads)

    server = Server(binary, database_url, cwd=REPO_ROOT, tmpdir=tmpdir)
    interrupt = {"count": 0}

    def handle_signal(signum, frame):  # pragma: no cover - interactive
        interrupt["count"] += 1
        raise KeyboardInterrupt

    old_int = signal.signal(signal.SIGINT, handle_signal)
    old_term = signal.signal(signal.SIGTERM, handle_signal)

    print(f"Database : {db_path if db_path else database_url}", flush=True)
    print(f"Binary   : {binary}", flush=True)
    print(f"Base URL : {base}\n", flush=True)

    try:
        Server._assert_port_free(host, port)
    except SystemExit as exc:
        print(str(exc), file=sys.stderr)
        return 2

    exit_code = 0
    guard = DatabaseGuard(db_path, keep=args.keep_db)
    try:
        with guard:
            server.start(host, port, timeout=args.start_timeout)
            try:
                ApiTest(suite, args).run()
            finally:
                server.stop()
    except KeyboardInterrupt:
        print("\nInterrupted.", file=sys.stderr)
        exit_code = 2
    except SystemExit as exc:
        print(str(exc), file=sys.stderr)
        exit_code = 2
    finally:
        signal.signal(signal.SIGINT, old_int)
        signal.signal(signal.SIGTERM, old_term)
        try:
            server.stop()
        except Exception:
            pass
        if not args.keep_uploads:
            removed = cleanup_tree(uploads, uploads_before)
            if removed and args.verbose:
                print(f"Cleaned {len(removed)} file(s) from uploads/")
        # Restore, even when the harness crashed before the context manager did.
        guard.restore()

    # ------------------------------------------------------------------ #
    # Report
    # ------------------------------------------------------------------ #
    failures = suite.failures()
    print("\n" + "=" * 72)
    print("SUMMARY")
    print("=" * 72)
    if failures:
        print(f"\n{len(failures)} failing check(s):\n")
        for failure in failures:
            print(f"  [{failure.category}] {failure.name}")
            if failure.detail:
                for line in failure.detail.strip().splitlines()[:6]:
                    print(f"      {line}")
    passes = sum(1 for r in suite.results if r.status == "PASS")
    skips = sum(1 for r in suite.results if r.status == "SKIP")
    functional_failures = [f for f in failures if f.category != "security"]
    security_failures = [f for f in failures if f.category == "security"]
    print(
        f"\nchecks: {suite.checks}   passed: {passes}   failed: {len(failures)}"
        f"   (functional: {len(functional_failures)}, security: {len(security_failures)})"
        f"   skipped: {skips}"
    )
    if exit_code == 0 and failures:
        exit_code = 1
    if exit_code == 0:
        print("\nAll checks passed.")
    elif exit_code == 1:
        print("\nVerification FAILED.")
    shutil.rmtree(tmpdir, ignore_errors=True)
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
