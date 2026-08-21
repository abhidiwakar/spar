use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

pub struct Db(pub Mutex<Connection>);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub python_path: String,
    pub node_path: String,
    pub default_language: String,
    pub daily_goal: i64,
    pub timer_enabled: bool,
    #[serde(default)]
    pub openai_api_key: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemState {
    pub problem_id: String,
    pub status: String,
    pub first_accepted_at: Option<String>,
    pub last_attempt_at: Option<String>,
    pub used_hint: bool,
    pub review_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attempt {
    pub id: String,
    pub problem_id: String,
    pub language: String,
    pub code: String,
    pub passed: bool,
    pub used_hint: bool,
    pub duration_ms: Option<i64>,
    pub created_at: String,
    pub verdict: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyStat {
    pub date: String,
    pub xp: i64,
    pub goal_met: bool,
    pub accepted_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressSnapshot {
    pub settings: Settings,
    pub problem_states: Vec<ProblemState>,
    pub attempts: Vec<Attempt>,
    pub daily: Vec<DailyStat>,
    pub drafts: Vec<Draft>,
    pub notes: Vec<Note>,
    pub streak: i64,
    pub xp_today: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub problem_id: String,
    pub language: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub problem_id: String,
    pub body: String,
}

pub fn open(path: PathBuf) -> rusqlite::Result<Db> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS drafts (
            problem_id TEXT NOT NULL,
            language TEXT NOT NULL,
            code TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (problem_id, language)
        );
        CREATE TABLE IF NOT EXISTS attempts (
            id TEXT PRIMARY KEY,
            problem_id TEXT NOT NULL,
            language TEXT NOT NULL,
            code TEXT NOT NULL,
            passed INTEGER NOT NULL,
            used_hint INTEGER NOT NULL DEFAULT 0,
            duration_ms INTEGER,
            created_at TEXT NOT NULL,
            verdict TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS problem_state (
            problem_id TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            first_accepted_at TEXT,
            last_attempt_at TEXT,
            used_hint INTEGER NOT NULL DEFAULT 0,
            review_at TEXT
        );
        CREATE TABLE IF NOT EXISTS daily_stats (
            date TEXT PRIMARY KEY,
            xp INTEGER NOT NULL DEFAULT 0,
            goal_met INTEGER NOT NULL DEFAULT 0,
            accepted_count INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS notes (
            problem_id TEXT PRIMARY KEY,
            body TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;
    Ok(Db(Mutex::new(conn)))
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn get_setting(conn: &Connection, key: &str, default: &str) -> String {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
        .unwrap_or_else(|_| default.to_string())
}

pub fn load_settings(conn: &Connection) -> Settings {
    Settings {
        python_path: get_setting(conn, "python_path", ""),
        node_path: get_setting(conn, "node_path", ""),
        default_language: get_setting(conn, "default_language", "python"),
        daily_goal: get_setting(conn, "daily_goal", "1").parse().unwrap_or(1),
        timer_enabled: get_setting(conn, "timer_enabled", "true") == "true",
        openai_api_key: get_setting(conn, "openai_api_key", ""),
    }
}

pub fn save_settings(conn: &Connection, s: &Settings) -> rusqlite::Result<()> {
    let pairs = [
        ("python_path", s.python_path.as_str()),
        ("node_path", s.node_path.as_str()),
        ("default_language", s.default_language.as_str()),
        ("daily_goal", &s.daily_goal.to_string()),
        ("timer_enabled", if s.timer_enabled { "true" } else { "false" }),
        ("openai_api_key", s.openai_api_key.as_str()),
    ];
    for (k, v) in pairs {
        conn.execute(
            "INSERT INTO settings(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![k, v],
        )?;
    }
    Ok(())
}

pub fn is_accepted(conn: &Connection, problem_id: &str) -> bool {
    conn.query_row(
        "SELECT status FROM problem_state WHERE problem_id = ?1",
        [problem_id],
        |r| r.get::<_, String>(0),
    )
    .ok()
    .is_some_and(|status| status == "accepted")
}

pub fn save_draft(conn: &Connection, problem_id: &str, language: &str, code: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO drafts(problem_id, language, code, updated_at) VALUES(?1, ?2, ?3, ?4)
         ON CONFLICT(problem_id, language) DO UPDATE SET code = excluded.code, updated_at = excluded.updated_at",
        params![problem_id, language, code, now()],
    )?;
    Ok(())
}

pub fn save_note(conn: &Connection, problem_id: &str, body: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO notes(problem_id, body, updated_at) VALUES(?1, ?2, ?3)
         ON CONFLICT(problem_id) DO UPDATE SET body = excluded.body, updated_at = excluded.updated_at",
        params![problem_id, body, now()],
    )?;
    Ok(())
}

fn xp_for(difficulty: &str, first: bool, used_hint: bool, review: bool) -> i64 {
    if review {
        return 30;
    }
    if !first {
        return 10;
    }
    let base = match difficulty {
        "easy" => 50,
        "medium" => 80,
        "hard" => 120,
        _ => 50,
    };
    if used_hint {
        base
    } else {
        base + 20
    }
}

pub fn record_attempt(
    conn: &Connection,
    problem_id: &str,
    language: &str,
    code: &str,
    passed: bool,
    used_hint: bool,
    duration_ms: Option<i64>,
    verdict: &str,
    difficulty: &str,
) -> rusqlite::Result<i64> {
    let id = Uuid::new_v4().to_string();
    let created = now();
    conn.execute(
        "INSERT INTO attempts(id, problem_id, language, code, passed, used_hint, duration_ms, created_at, verdict)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![id, problem_id, language, code, passed as i64, used_hint as i64, duration_ms, created, verdict],
    )?;

    let existing: Option<(String, Option<String>, i64, Option<String>)> = conn
        .query_row(
            "SELECT status, first_accepted_at, used_hint, review_at FROM problem_state WHERE problem_id = ?1",
            [problem_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .ok();

    let was_accepted = existing
        .as_ref()
        .map(|(s, first, _, _)| s == "accepted" || first.is_some())
        .unwrap_or(false);
    let prev_hint = existing.as_ref().map(|(_, _, h, _)| *h > 0).unwrap_or(false);
    let hint_flag = used_hint || prev_hint;
    let is_review = existing
        .as_ref()
        .map(|(_, _, _, review_at)| review_at.is_some() && was_accepted)
        .unwrap_or(false);

    let mut awarded = 0i64;
    if passed && verdict == "accepted" {
        let first = !was_accepted;
        awarded = xp_for(difficulty, first, hint_flag, is_review && !first);
        let review_at = chrono::Local::now() + chrono::TimeDelta::days(3);
        let review_at2 = chrono::Local::now() + chrono::TimeDelta::days(7);
        let review_s = if first {
            Some(review_at.to_rfc3339())
        } else {
            Some(review_at2.to_rfc3339())
        };
        conn.execute(
            "INSERT INTO problem_state(problem_id, status, first_accepted_at, last_attempt_at, used_hint, review_at)
             VALUES(?1, 'accepted', ?2, ?3, ?4, ?5)
             ON CONFLICT(problem_id) DO UPDATE SET
                status = 'accepted',
                first_accepted_at = COALESCE(problem_state.first_accepted_at, excluded.first_accepted_at),
                last_attempt_at = excluded.last_attempt_at,
                used_hint = MAX(problem_state.used_hint, excluded.used_hint),
                review_at = excluded.review_at",
            params![problem_id, created, created, hint_flag as i64, review_s],
        )?;
        let day = today();
        conn.execute(
            "INSERT INTO daily_stats(date, xp, goal_met, accepted_count) VALUES(?1, ?2, 0, 1)
             ON CONFLICT(date) DO UPDATE SET
                xp = daily_stats.xp + excluded.xp,
                accepted_count = daily_stats.accepted_count + 1",
            params![day, awarded],
        )?;
        let (xp, count, goal): (i64, i64, i64) = {
            let count = conn.query_row(
                "SELECT xp, accepted_count FROM daily_stats WHERE date = ?1",
                [&day],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
            )?;
            let goal: i64 = get_setting(conn, "daily_goal", "1").parse().unwrap_or(1);
            (count.0, count.1, goal)
        };
        let _ = xp;
        if count >= goal {
            conn.execute("UPDATE daily_stats SET goal_met = 1 WHERE date = ?1", [&day])?;
        }
    } else {
        conn.execute(
            "INSERT INTO problem_state(problem_id, status, first_accepted_at, last_attempt_at, used_hint, review_at)
             VALUES(?1, 'attempted', NULL, ?2, ?3, NULL)
             ON CONFLICT(problem_id) DO UPDATE SET
                status = CASE WHEN problem_state.status = 'accepted' THEN 'accepted' ELSE 'attempted' END,
                last_attempt_at = excluded.last_attempt_at,
                used_hint = MAX(problem_state.used_hint, excluded.used_hint)",
            params![problem_id, created, hint_flag as i64],
        )?;
    }
    Ok(awarded)
}

pub fn streak(conn: &Connection) -> i64 {
    let mut dates: Vec<String> = conn
        .prepare("SELECT date FROM daily_stats WHERE goal_met = 1 ORDER BY date DESC")
        .ok()
        .and_then(|mut s| {
            s.query_map([], |r| r.get(0))
                .ok()
                .map(|rows| rows.filter_map(|x| x.ok()).collect())
        })
        .unwrap_or_default();
    if dates.is_empty() {
        return 0;
    }
    let today_s = today();
    let start = if dates[0] == today_s {
        chrono::Local::now().date_naive()
    } else {
        chrono::Local::now().date_naive() - chrono::TimeDelta::days(1)
    };
    let set: std::collections::HashSet<String> = dates.drain(..).collect();
    let mut n = 0i64;
        let mut d = start;
        loop {
            let key = d.format("%Y-%m-%d").to_string();
            if set.contains(&key) {
                n += 1;
                d -= chrono::TimeDelta::days(1);
            } else {
                break;
            }
        }
    n
}

pub fn snapshot(conn: &Connection) -> rusqlite::Result<ProgressSnapshot> {
    let settings = load_settings(conn);
    let mut problem_states = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT problem_id, status, first_accepted_at, last_attempt_at, used_hint, review_at FROM problem_state",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(ProblemState {
                problem_id: r.get(0)?,
                status: r.get(1)?,
                first_accepted_at: r.get(2)?,
                last_attempt_at: r.get(3)?,
                used_hint: r.get::<_, i64>(4)? > 0,
                review_at: r.get(5)?,
            })
        })?;
        for row in rows {
            problem_states.push(row?);
        }
    }
    let mut attempts = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT id, problem_id, language, code, passed, used_hint, duration_ms, created_at, verdict
             FROM attempts ORDER BY created_at DESC LIMIT 200",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Attempt {
                id: r.get(0)?,
                problem_id: r.get(1)?,
                language: r.get(2)?,
                code: r.get(3)?,
                passed: r.get::<_, i64>(4)? > 0,
                used_hint: r.get::<_, i64>(5)? > 0,
                duration_ms: r.get(6)?,
                created_at: r.get(7)?,
                verdict: r.get(8)?,
            })
        })?;
        for row in rows {
            attempts.push(row?);
        }
    }
    let mut daily = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT date, xp, goal_met, accepted_count FROM daily_stats ORDER BY date DESC LIMIT 120",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(DailyStat {
                date: r.get(0)?,
                xp: r.get(1)?,
                goal_met: r.get::<_, i64>(2)? > 0,
                accepted_count: r.get(3)?,
            })
        })?;
        for row in rows {
            daily.push(row?);
        }
    }
    let mut drafts = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT problem_id, language, code FROM drafts")?;
        let rows = stmt.query_map([], |r| {
            Ok(Draft {
                problem_id: r.get(0)?,
                language: r.get(1)?,
                code: r.get(2)?,
            })
        })?;
        for row in rows {
            drafts.push(row?);
        }
    }
    let mut notes = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT problem_id, body FROM notes")?;
        let rows = stmt.query_map([], |r| {
            Ok(Note {
                problem_id: r.get(0)?,
                body: r.get(1)?,
            })
        })?;
        for row in rows {
            notes.push(row?);
        }
    }
    let xp_today = daily
        .iter()
        .find(|d| d.date == today())
        .map(|d| d.xp)
        .unwrap_or(0);
    Ok(ProgressSnapshot {
        settings,
        problem_states,
        attempts,
        daily,
        drafts,
        notes,
        streak: streak(conn),
        xp_today,
    })
}
