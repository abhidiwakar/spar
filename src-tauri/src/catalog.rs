use crate::judge::TestCase;
use futures_util::future::try_join_all;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const RAW_BASE: &str = "https://raw.githubusercontent.com/abhidiwakar/spar/main/content";
const MAX_FILE_BYTES: usize = 2 * 1024 * 1024;
const MAX_PROBLEMS: usize = 200;
const FETCH_CONCURRENCY: usize = 8;
const HTTP_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Unit {
    pub id: String,
    pub week: i64,
    pub title: String,
    pub subtitle: String,
    pub briefing: String,
    pub kind: String,
    pub core_ids: Vec<String>,
    pub stretch_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timer_minutes: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogProblem {
    pub id: String,
    pub title: String,
    pub difficulty: String,
    pub tags: Vec<String>,
    pub unit_id: String,
    pub kind: String,
    pub order: i64,
    pub tip: String,
    pub statement: String,
    pub examples: Vec<Example>,
    pub constraints: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_up: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editorial: Option<String>,
    pub mode: String,
    pub entry: CatalogEntry,
    pub param_names: Vec<String>,
    pub helpers: Vec<String>,
    pub starter: Starter,
    pub tests: CatalogTests,
    #[serde(default)]
    pub cpp_types: Option<CatalogCppTypes>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Example {
    pub input: String,
    pub output: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CatalogEntry {
    pub python: String,
    pub javascript: String,
    #[serde(default)]
    pub cpp: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CatalogCppTypes {
    #[serde(rename = "return")]
    pub return_type: String,
    pub params: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Starter {
    pub python: String,
    pub javascript: String,
    #[serde(default)]
    pub cpp: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CatalogTests {
    pub visible: Vec<TestCase>,
    #[serde(default)]
    pub hidden: Vec<TestCase>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSnapshot {
    pub units: Vec<Unit>,
    pub problems: Vec<Value>,
}

#[derive(Clone)]
enum CatalogMode {
    Dev,
    Remote { cache_dir: PathBuf },
}

#[derive(Default)]
struct CatalogData {
    units: Vec<Unit>,
    problems: HashMap<String, CatalogProblem>,
}

/// Shared problem catalog: local `content/` in dev, app-data cache + GitHub in packaged builds.
pub struct Catalog {
    mode: CatalogMode,
    data: Arc<Mutex<CatalogData>>,
}

impl Catalog {
    pub fn open(app_data_dir: &Path) -> Result<Self, String> {
        let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../content");
        if dev.join("problems").is_dir() {
            let catalog = Self {
                mode: CatalogMode::Dev,
                data: Arc::new(Mutex::new(CatalogData::default())),
            };
            catalog.load_from_content_dir(&dev)?;
            return Ok(catalog);
        }

        let cache_dir = app_data_dir.join("catalog");
        let catalog = Self {
            mode: CatalogMode::Remote { cache_dir: cache_dir.clone() },
            data: Arc::new(Mutex::new(CatalogData::default())),
        };
        if cache_dir.join("units.json").is_file() {
            if let Err(e) = catalog.load_from_content_dir(&cache_dir) {
                // Corrupt cache: treat as empty so first-launch setup can re-fetch.
                let _ = std::fs::remove_dir_all(&cache_dir);
                *catalog.data.lock().map_err(|e| e.to_string())? = CatalogData::default();
                eprintln!("catalog cache ignored: {e}");
            }
        }
        Ok(catalog)
    }

    pub fn is_dev(&self) -> bool {
        matches!(self.mode, CatalogMode::Dev)
    }

    pub fn load_problem(&self, id: &str) -> Result<CatalogProblem, String> {
        if !is_safe_problem_id(id) {
            return Err("Unknown problem.".into());
        }
        let data = self.data.lock().map_err(|e| e.to_string())?;
        data.problems
            .get(id)
            .cloned()
            .ok_or_else(|| {
                if data.units.is_empty() {
                    "Problem catalog is still downloading.".into()
                } else {
                    "Unknown problem.".into()
                }
            })
    }

    pub fn snapshot(&self) -> Result<Option<CatalogSnapshot>, String> {
        let data = self.data.lock().map_err(|e| e.to_string())?;
        if data.units.is_empty() || data.problems.is_empty() {
            return Ok(None);
        }
        Ok(Some(sanitize_snapshot(&data)))
    }

    /// Download units + problems from GitHub and replace the cache. No-op in dev.
    pub async fn refresh(&self) -> Result<(), String> {
        let CatalogMode::Remote { cache_dir } = &self.mode else {
            return Ok(());
        };
        let cache_dir = cache_dir.clone();
        let fetched = fetch_remote_catalog().await?;
        write_cache(&cache_dir, &fetched)?;
        let mut data = self.data.lock().map_err(|e| e.to_string())?;
        *data = fetched;
        Ok(())
    }

    fn load_from_content_dir(&self, content_dir: &Path) -> Result<(), String> {
        let loaded = read_content_dir(content_dir)?;
        let mut data = self.data.lock().map_err(|e| e.to_string())?;
        *data = loaded;
        Ok(())
    }
}

pub fn is_safe_problem_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

pub fn problem_url(id: &str) -> Result<String, String> {
    if !is_safe_problem_id(id) {
        return Err("Unknown problem.".into());
    }
    Ok(format!("{RAW_BASE}/problems/{id}.json"))
}

pub fn units_url() -> String {
    format!("{RAW_BASE}/units.json")
}

#[cfg(test)]
pub fn load_problem(content_dir: &Path, id: &str) -> Result<CatalogProblem, String> {
    if !is_safe_problem_id(id) {
        return Err("Unknown problem.".into());
    }
    let path = content_dir.join("problems").join(format!("{id}.json"));
    let raw = std::fs::read_to_string(&path).map_err(|_| "Unknown problem.".to_string())?;
    parse_problem(&raw, id)
}

fn read_content_dir(content_dir: &Path) -> Result<CatalogData, String> {
    let units_raw = std::fs::read_to_string(content_dir.join("units.json"))
        .map_err(|e| format!("Problem catalog is unreadable: {e}"))?;
    let units = parse_units(&units_raw)?;
    let ids = collect_problem_ids(&units)?;
    let mut problems = HashMap::new();
    for id in ids {
        let path = content_dir.join("problems").join(format!("{id}.json"));
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| format!("Problem `{id}` is missing: {e}"))?;
        let problem = parse_problem(&raw, &id)?;
        problems.insert(id, problem);
    }
    Ok(CatalogData { units, problems })
}

fn parse_units(raw: &str) -> Result<Vec<Unit>, String> {
    serde_json::from_str(raw).map_err(|e| format!("Units catalog is unreadable: {e}"))
}

fn collect_problem_ids(units: &[Unit]) -> Result<Vec<String>, String> {
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    for unit in units {
        for id in unit.core_ids.iter().chain(unit.stretch_ids.iter()) {
            if !is_safe_problem_id(id) {
                return Err(format!("Invalid problem id `{id}` in units."));
            }
            if seen.insert(id.clone()) {
                ids.push(id.clone());
            }
        }
    }
    if ids.is_empty() {
        return Err("Units catalog lists no problems.".into());
    }
    if ids.len() > MAX_PROBLEMS {
        return Err(format!("Units catalog lists too many problems (max {MAX_PROBLEMS})."));
    }
    Ok(ids)
}

fn parse_problem(raw: &str, expected_id: &str) -> Result<CatalogProblem, String> {
    if raw.len() > MAX_FILE_BYTES {
        return Err(format!("Problem `{expected_id}` is too large."));
    }
    let problem: CatalogProblem =
        serde_json::from_str(raw).map_err(|e| format!("Problem catalog is unreadable: {e}"))?;
    if problem.id != expected_id {
        return Err("Unknown problem.".into());
    }
    Ok(problem)
}

fn sanitize_snapshot(data: &CatalogData) -> CatalogSnapshot {
    let mut problems: Vec<Value> = data
        .problems
        .values()
        .map(|p| {
            let mut v = serde_json::to_value(p).unwrap_or(Value::Null);
            if let Value::Object(ref mut map) = v {
                map.remove("editorial");
                if let Some(Value::Object(tests)) = map.get_mut("tests") {
                    tests.insert("hidden".into(), Value::Array(vec![]));
                }
            }
            v
        })
        .collect();
    problems.sort_by(|a, b| {
        let au = a.get("unitId").and_then(|x| x.as_str()).unwrap_or("");
        let bu = b.get("unitId").and_then(|x| x.as_str()).unwrap_or("");
        let ao = a.get("order").and_then(|x| x.as_i64()).unwrap_or(0);
        let bo = b.get("order").and_then(|x| x.as_i64()).unwrap_or(0);
        au.cmp(bu).then(ao.cmp(&bo))
    });
    let mut units = data.units.clone();
    units.sort_by_key(|u| u.week);
    CatalogSnapshot { units, problems }
}

fn write_cache(cache_dir: &Path, data: &CatalogData) -> Result<(), String> {
    let parent = cache_dir
        .parent()
        .ok_or_else(|| "Invalid catalog cache path.".to_string())?;
    let staging = parent.join(format!(
        "catalog.tmp.{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(staging.join("problems")).map_err(|e| e.to_string())?;
    let units_raw = serde_json::to_string_pretty(&data.units).map_err(|e| e.to_string())?;
    std::fs::write(staging.join("units.json"), units_raw).map_err(|e| e.to_string())?;
    for (id, problem) in &data.problems {
        let raw = serde_json::to_string_pretty(problem).map_err(|e| e.to_string())?;
        std::fs::write(staging.join("problems").join(format!("{id}.json")), raw)
            .map_err(|e| e.to_string())?;
    }
    let _ = std::fs::remove_dir_all(cache_dir);
    std::fs::rename(&staging, cache_dir).map_err(|e| {
        let _ = std::fs::remove_dir_all(&staging);
        format!("Could not install catalog cache: {e}")
    })?;
    Ok(())
}

async fn fetch_remote_catalog() -> Result<CatalogData, String> {
    let client = reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("Spar/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;

    let units_raw = fetch_text(&client, &units_url()).await?;
    let units = parse_units(&units_raw)?;
    let ids = collect_problem_ids(&units)?;

    let mut problems = HashMap::new();
    for chunk in ids.chunks(FETCH_CONCURRENCY) {
        let futs: Vec<_> = chunk
            .iter()
            .map(|id| {
                let client = client.clone();
                let id = id.clone();
                async move {
                    let url = problem_url(&id)?;
                    let raw = fetch_text(&client, &url).await?;
                    let problem = parse_problem(&raw, &id)?;
                    Ok::<_, String>((id, problem))
                }
            })
            .collect();
        for (id, problem) in try_join_all(futs).await? {
            problems.insert(id, problem);
        }
    }
    Ok(CatalogData { units, problems })
}

async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| e.to_string())?;
    if parsed.host_str() != Some("raw.githubusercontent.com") {
        return Err("Unexpected catalog host.".into());
    }
    let response = client
        .get(parsed)
        .send()
        .await
        .map_err(|e| format!("Could not download catalog: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Could not download catalog (HTTP {}).",
            response.status().as_u16()
        ));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Could not download catalog: {e}"))?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err("Catalog file is too large.".into());
    }
    String::from_utf8(bytes.to_vec()).map_err(|_| "Catalog file is not UTF-8.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../content")
    }

    #[test]
    fn rejects_path_escape() {
        let dir = content_dir();
        assert!(load_problem(&dir, "../secrets").is_err());
        assert!(load_problem(&dir, "two-sum/../../Cargo.toml").is_err());
        assert!(problem_url("../secrets").is_err());
    }

    #[test]
    fn loads_two_sum() {
        let p = load_problem(&content_dir(), "two-sum").unwrap();
        assert_eq!(p.id, "two-sum");
        assert_eq!(p.difficulty, "easy");
        assert!(!p.tests.visible.is_empty());
        assert!(!p.tests.hidden.is_empty());
        assert!(p.editorial.as_ref().is_some_and(|e| !e.is_empty()));
    }

    #[test]
    fn rejects_mismatched_catalog_id() {
        let raw = r#"{
            "id": "other",
            "title": "Other",
            "difficulty": "easy",
            "tags": [],
            "unitId": "unit-01",
            "kind": "core",
            "order": 1,
            "tip": "",
            "statement": "",
            "examples": [],
            "constraints": [],
            "mode": "function",
            "entry": { "python": "f", "javascript": "f", "cpp": "f" },
            "paramNames": [],
            "helpers": [],
            "starter": { "python": "", "javascript": "" },
            "tests": { "visible": [] }
        }"#;
        assert!(parse_problem(raw, "two-sum").is_err());
    }

    #[test]
    fn problem_url_is_pinned() {
        assert_eq!(
            problem_url("two-sum").unwrap(),
            "https://raw.githubusercontent.com/abhidiwakar/spar/main/content/problems/two-sum.json"
        );
        assert_eq!(
            units_url(),
            "https://raw.githubusercontent.com/abhidiwakar/spar/main/content/units.json"
        );
    }

    #[test]
    fn sanitize_strips_secrets() {
        let dir = content_dir();
        let catalog = Catalog {
            mode: CatalogMode::Dev,
            data: Arc::new(Mutex::new(CatalogData::default())),
        };
        catalog.load_from_content_dir(&dir).unwrap();
        let snap = catalog.snapshot().unwrap().unwrap();
        assert!(!snap.units.is_empty());
        assert!(!snap.problems.is_empty());
        for p in &snap.problems {
            assert!(p.get("editorial").is_none());
            let hidden = p
                .pointer("/tests/hidden")
                .and_then(|v| v.as_array())
                .unwrap();
            assert!(hidden.is_empty());
            assert!(p
                .pointer("/tests/visible")
                .and_then(|v| v.as_array())
                .is_some_and(|v| !v.is_empty() || p.get("id").is_some()));
        }
        let two = snap
            .problems
            .iter()
            .find(|p| p.get("id").and_then(|x| x.as_str()) == Some("two-sum"))
            .unwrap();
        assert!(!two
            .pointer("/tests/visible")
            .and_then(|v| v.as_array())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn empty_catalog_is_not_ready() {
        let catalog = Catalog {
            mode: CatalogMode::Remote {
                cache_dir: PathBuf::from("/tmp/spar-catalog-test-empty"),
            },
            data: Arc::new(Mutex::new(CatalogData::default())),
        };
        assert!(catalog.snapshot().unwrap().is_none());
        assert_eq!(
            catalog.load_problem("two-sum").unwrap_err(),
            "Problem catalog is still downloading."
        );
    }

    #[test]
    fn cache_roundtrip_loads_problems() {
        let dir = content_dir();
        let src = read_content_dir(&dir).unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "spar-catalog-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        write_cache(&tmp, &src).unwrap();
        let catalog = Catalog {
            mode: CatalogMode::Remote {
                cache_dir: tmp.clone(),
            },
            data: Arc::new(Mutex::new(CatalogData::default())),
        };
        catalog.load_from_content_dir(&tmp).unwrap();
        let p = catalog.load_problem("two-sum").unwrap();
        assert_eq!(p.id, "two-sum");
        assert!(!p.tests.hidden.is_empty());
        let snap = catalog.snapshot().unwrap().unwrap();
        assert!(snap.problems.iter().all(|p| p.get("editorial").is_none()));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn collect_ids_dedupes_and_rejects_bad() {
        let units = parse_units(
            r#"[{"id":"u","week":1,"title":"t","subtitle":"s","briefing":"b","kind":"standard","coreIds":["two-sum","two-sum"],"stretchIds":["valid-parentheses"]}]"#,
        )
        .unwrap();
        let ids = collect_problem_ids(&units).unwrap();
        assert_eq!(ids, vec!["two-sum", "valid-parentheses"]);
        let bad = parse_units(
            r#"[{"id":"u","week":1,"title":"t","subtitle":"s","briefing":"b","kind":"standard","coreIds":["../x"],"stretchIds":[]}]"#,
        )
        .unwrap();
        assert!(collect_problem_ids(&bad).is_err());
    }
}
