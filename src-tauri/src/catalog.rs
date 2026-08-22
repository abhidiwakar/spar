use crate::judge::TestCase;
use include_dir::{include_dir, Dir};
use serde::Deserialize;
use std::path::{Path, PathBuf};

static EMBEDDED_CONTENT: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../content");

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogProblem {
    pub id: String,
    pub difficulty: String,
    pub mode: String,
    pub entry: CatalogEntry,
    pub param_names: Vec<String>,
    pub helpers: Vec<String>,
    pub editorial: Option<String>,
    pub tests: CatalogTests,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogEntry {
    pub python: String,
    pub javascript: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogTests {
    pub visible: Vec<TestCase>,
    #[serde(default)]
    pub hidden: Vec<TestCase>,
}

/// Dev/test builds read `content/` from disk so problem edits apply without a rebuild.
/// Packaged apps use the catalog compiled into the binary — nothing is copied into Resources.
pub struct ContentDir {
    fs: Option<PathBuf>,
}

impl ContentDir {
    pub fn resolve() -> Self {
        let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../content");
        Self {
            fs: dev.join("problems").is_dir().then_some(dev),
        }
    }

    pub fn load_problem(&self, id: &str) -> Result<CatalogProblem, String> {
        load_from_fs(self.fs.as_deref(), id).or_else(|_| load_embedded(id))
    }
}

pub fn is_safe_problem_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
pub fn load_problem(content_dir: &Path, id: &str) -> Result<CatalogProblem, String> {
    load_from_fs(Some(content_dir), id)
}

fn load_from_fs(content_dir: Option<&Path>, id: &str) -> Result<CatalogProblem, String> {
    if !is_safe_problem_id(id) {
        return Err("Unknown problem.".into());
    }
    let dir = content_dir.ok_or_else(|| "Unknown problem.".to_string())?;
    let path = dir.join("problems").join(format!("{id}.json"));
    let raw = std::fs::read_to_string(&path).map_err(|_| "Unknown problem.".to_string())?;
    parse_problem(&raw, id)
}

fn load_embedded(id: &str) -> Result<CatalogProblem, String> {
    if !is_safe_problem_id(id) {
        return Err("Unknown problem.".into());
    }
    let file = EMBEDDED_CONTENT
        .get_file(&format!("problems/{id}.json"))
        .ok_or_else(|| "Unknown problem.".to_string())?;
    let raw = file
        .contents_utf8()
        .ok_or_else(|| "Problem catalog is unreadable.".to_string())?;
    parse_problem(raw, id)
}

fn parse_problem(raw: &str, expected_id: &str) -> Result<CatalogProblem, String> {
    let problem: CatalogProblem =
        serde_json::from_str(raw).map_err(|e| format!("Problem catalog is unreadable: {e}"))?;
    if problem.id != expected_id {
        return Err("Unknown problem.".into());
    }
    Ok(problem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_escape() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../content");
        assert!(load_problem(&dir, "../secrets").is_err());
        assert!(load_problem(&dir, "two-sum/../../Cargo.toml").is_err());
        assert!(load_embedded("../secrets").is_err());
    }

    #[test]
    fn loads_two_sum() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../content");
        let p = load_problem(&dir, "two-sum").unwrap();
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
            "difficulty": "easy",
            "mode": "function",
            "entry": { "python": "f", "javascript": "f" },
            "paramNames": [],
            "helpers": [],
            "tests": { "visible": [] }
        }"#;
        assert!(parse_problem(raw, "two-sum").is_err());
    }

    #[test]
    fn embedded_catalog_matches_disk() {
        let from_embed = load_embedded("two-sum").unwrap();
        let from_fs = ContentDir::resolve().load_problem("two-sum").unwrap();
        assert_eq!(from_embed.id, from_fs.id);
        assert_eq!(from_embed.tests.hidden.len(), from_fs.tests.hidden.len());
        assert_eq!(from_embed.editorial, from_fs.editorial);
    }
}
