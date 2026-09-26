mod catalog;
mod judge;
mod ollama;
mod progress;
mod review;

use catalog::{Catalog, CatalogSnapshot};
use judge::{JudgeOutput, RunRequest};
use progress::{Db, Settings};
use serde::Deserialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Runtimes {
    python: Option<String>,
    node: Option<String>,
    cpp: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientRun {
    problem_id: String,
    language: String,
    source: String,
    kind: String,
    python_path: Option<String>,
    node_path: Option<String>,
    cpp_path: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubmitIn {
    problem_id: String,
    language: String,
    source: String,
    duration_ms: Option<i64>,
    python_path: Option<String>,
    node_path: Option<String>,
    cpp_path: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SubmitResult {
    output: JudgeOutput,
    xp: i64,
}

#[tauri::command]
fn detect_runtimes(
    python_path: Option<String>,
    node_path: Option<String>,
    cpp_path: Option<String>,
) -> Runtimes {
    Runtimes {
        python: judge::detect_python(python_path.as_deref()),
        node: judge::detect_node(node_path.as_deref()),
        cpp: judge::detect_cpp(cpp_path.as_deref()),
    }
}

fn require_problem_id(id: &str) -> Result<(), String> {
    if catalog::is_safe_problem_id(id) {
        Ok(())
    } else {
        Err("Unknown problem.".into())
    }
}

fn assemble_run(content: &Catalog, req: &ClientRun) -> Result<RunRequest, String> {
    let problem = content.load_problem(&req.problem_id)?;
    let submit = req.kind == "submit";
    let mut tests = problem.tests.visible;
    if submit {
        tests.extend(problem.tests.hidden);
    }
    let entry = match req.language.as_str() {
        "python" => problem.entry.python,
        "javascript" => problem.entry.javascript,
        "cpp" => problem
            .entry
            .cpp
            .ok_or_else(|| "This problem has no C++ entry point.".to_string())?,
        other => return Err(format!("Unsupported language: {other}")),
    };
    let cpp_types = problem.cpp_types.map(|t| judge::CppTypes {
        return_type: t.return_type,
        params: t.params,
    });
    Ok(RunRequest {
        language: req.language.clone(),
        source: req.source.clone(),
        entry,
        mode: problem.mode,
        helpers: problem.helpers,
        param_names: problem.param_names,
        tests,
        python_path: req.python_path.clone(),
        node_path: req.node_path.clone(),
        cpp_path: req.cpp_path.clone(),
        cpp_types,
        submit,
    })
}

#[tauri::command]
async fn load_catalog(
    app: AppHandle,
    catalog: State<'_, Catalog>,
) -> Result<CatalogSnapshot, String> {
    if catalog.is_dev() {
        return catalog
            .snapshot()?
            .ok_or_else(|| "Local content/ catalog is empty.".into());
    }

    if let Some(snap) = catalog.snapshot()? {
        let app2 = app.clone();
        tauri::async_runtime::spawn(async move {
            let catalog = app2.state::<Catalog>();
            match catalog.refresh().await {
                Ok(()) => {
                    if let Ok(Some(updated)) = catalog.snapshot() {
                        let _ = app2.emit("catalog-updated", updated);
                    }
                }
                Err(e) => eprintln!("catalog refresh failed: {e}"),
            }
        });
        return Ok(snap);
    }

    catalog.refresh().await?;
    catalog
        .snapshot()?
        .ok_or_else(|| "Problem catalog is empty.".into())
}

#[tauri::command]
async fn run_tests(content: State<'_, Catalog>, req: ClientRun) -> Result<JudgeOutput, String> {
    if req.kind != "run" {
        return Err("Use submit_solution to submit.".into());
    }
    let assembled = assemble_run(&content, &req)?;
    tauri::async_runtime::spawn_blocking(move || judge::run(assembled))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn submit_solution(
    db: State<'_, Db>,
    content: State<'_, Catalog>,
    req: SubmitIn,
) -> Result<SubmitResult, String> {
    let problem = content.load_problem(&req.problem_id)?;
    let assembled = assemble_run(
        &content,
        &ClientRun {
            problem_id: req.problem_id.clone(),
            language: req.language.clone(),
            source: req.source.clone(),
            kind: "submit".into(),
            python_path: req.python_path.clone(),
            node_path: req.node_path.clone(),
            cpp_path: req.cpp_path.clone(),
        },
    )?;
    let output = tauri::async_runtime::spawn_blocking(move || judge::run(assembled))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let hinted = progress::used_hint(&conn, &req.problem_id);
    let xp = progress::record_attempt(
        &mut conn,
        &req.problem_id,
        &req.language,
        &req.source,
        output.verdict == "accepted",
        hinted,
        req.duration_ms,
        &output.verdict,
        &problem.difficulty,
    )
    .map_err(|e| e.to_string())?;
    Ok(SubmitResult { output, xp })
}

#[tauri::command]
fn load_progress(db: State<Db>) -> Result<progress::ProgressSnapshot, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    progress::snapshot(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_settings_cmd(db: State<Db>, mut settings: Settings) -> Result<(), String> {
    match ollama::validate_host(&settings.ollama_host) {
        Ok(host) => settings.ollama_host = host,
        Err(e) if settings.ai_provider == "ollama" => return Err(e),
        Err(_) => settings.ollama_host = "http://127.0.0.1:11434".into(),
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    progress::save_settings(&conn, &settings).map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_openai_key_cmd(db: State<Db>) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    progress::clear_openai_key(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_ollama_selection_cmd(
    db: State<Db>,
    provider: String,
    host: String,
    model: String,
) -> Result<(), String> {
    let host = ollama::validate_host(&host)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    progress::save_ollama_selection(&conn, &provider, &host, &model).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_draft_cmd(
    db: State<Db>,
    problem_id: String,
    language: String,
    code: String,
) -> Result<(), String> {
    require_problem_id(&problem_id)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    progress::save_draft(&conn, &problem_id, &language, &code).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_note_cmd(db: State<Db>, problem_id: String, body: String) -> Result<(), String> {
    require_problem_id(&problem_id)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    progress::save_note(&conn, &problem_id, &body).map_err(|e| e.to_string())
}

#[tauri::command]
fn mark_hint_used_cmd(db: State<Db>, problem_id: String) -> Result<(), String> {
    require_problem_id(&problem_id)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    progress::mark_hint_used(&conn, &problem_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_editorial(
    db: State<Db>,
    content: State<Catalog>,
    problem_id: String,
) -> Result<String, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let unlocked = progress::is_accepted(&conn, &problem_id) || progress::used_hint(&conn, &problem_id);
    drop(conn);
    if !unlocked {
        return Err("Editorial unlocks after you submit, or if you opt into a hint.".into());
    }
    Ok(content
        .load_problem(&problem_id)?
        .editorial
        .unwrap_or_default())
}

#[tauri::command]
async fn review_solution(
    db: State<'_, progress::Db>,
    req: review::ReviewRequest,
) -> Result<String, String> {
    review::review_solution(db, req).await
}

#[tauri::command]
async fn analyze_attempt_complexity(
    db: State<'_, progress::Db>,
    attempt_id: String,
    force: Option<bool>,
) -> Result<review::ComplexityResult, String> {
    review::analyze_attempt_complexity(db, attempt_id, force.unwrap_or(false)).await
}

#[tauri::command]
async fn ollama_status(host: String) -> Result<ollama::OllamaStatus, String> {
    ollama::status(&host).await
}

#[tauri::command]
async fn ollama_pull_model(
    host: String,
    model: String,
    on_progress: tauri::ipc::Channel<ollama::OllamaPullProgress>,
) -> Result<(), String> {
    ollama::pull_model(&host, &model, on_progress).await
}

#[tauri::command]
fn ollama_cancel_pull() {
    ollama::cancel_pull();
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    ollama::open_external_url(&url)
}

fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app_data_dir(app.handle())?;
            let path = data_dir.join("learndsa.db");
            let db = progress::open(path).map_err(|e| e.to_string())?;
            app.manage(db);
            app.manage(Catalog::open(&data_dir).map_err(|e| e.to_string())?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            detect_runtimes,
            load_catalog,
            run_tests,
            submit_solution,
            load_progress,
            save_settings_cmd,
            clear_openai_key_cmd,
            save_ollama_selection_cmd,
            save_draft_cmd,
            save_note_cmd,
            mark_hint_used_cmd,
            load_editorial,
            review_solution,
            analyze_attempt_complexity,
            ollama_status,
            ollama_pull_model,
            ollama_cancel_pull,
            open_external_url
        ])
        .run(tauri::generate_context!())
        .expect("error while running Spar");
}
