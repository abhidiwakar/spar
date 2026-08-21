mod judge;
mod progress;
mod review;

use judge::RunRequest;
use progress::{Db, Settings};
use std::path::PathBuf;
use tauri::{Manager, State};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Runtimes {
    python: Option<String>,
    node: Option<String>,
}

#[tauri::command]
fn detect_runtimes(python_path: Option<String>, node_path: Option<String>) -> Runtimes {
    Runtimes {
        python: judge::detect_python(python_path.as_deref()),
        node: judge::detect_node(node_path.as_deref()),
    }
}

#[tauri::command]
fn run_tests(req: RunRequest) -> Result<judge::JudgeOutput, String> {
    judge::run(req).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_progress(db: State<Db>) -> Result<progress::ProgressSnapshot, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    progress::snapshot(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_settings_cmd(db: State<Db>, settings: Settings) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    progress::save_settings(&conn, &settings).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_draft_cmd(
    db: State<Db>,
    problem_id: String,
    language: String,
    code: String,
) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    progress::save_draft(&conn, &problem_id, &language, &code).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_note_cmd(db: State<Db>, problem_id: String, body: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    progress::save_note(&conn, &problem_id, &body).map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AttemptIn {
    problem_id: String,
    language: String,
    code: String,
    passed: bool,
    used_hint: bool,
    duration_ms: Option<i64>,
    verdict: String,
    difficulty: String,
}

#[tauri::command]
fn record_attempt_cmd(db: State<Db>, attempt: AttemptIn) -> Result<i64, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    progress::record_attempt(
        &conn,
        &attempt.problem_id,
        &attempt.language,
        &attempt.code,
        attempt.passed,
        attempt.used_hint,
        attempt.duration_ms,
        &attempt.verdict,
        &attempt.difficulty,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn review_solution(
    db: State<'_, progress::Db>,
    req: review::ReviewRequest,
) -> Result<String, String> {
    review::review_solution(db, req).await
}

fn db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("learndsa.db"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let path = db_path(app.handle())?;
            let db = progress::open(path).map_err(|e| e.to_string())?;
            app.manage(db);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            detect_runtimes,
            run_tests,
            load_progress,
            save_settings_cmd,
            save_draft_cmd,
            save_note_cmd,
            record_attempt_cmd,
            review_solution
        ])
        .run(tauri::generate_context!())
        .expect("error while running Spar");
}
