use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use wait_timeout::ChildExt;

const RUN_TIMEOUT: Duration = Duration::from_secs(8);
const SUBMIT_TIMEOUT: Duration = Duration::from_secs(15);

const PYTHON_PRELUDE: &str = r#"
from __future__ import annotations
from typing import Optional, List, Dict, Any

class ListNode:
    def __init__(self, val=0, next=None):
        self.val = val
        self.next = next

class TreeNode:
    def __init__(self, val=0, left=None, right=None):
        self.val = val
        self.left = left
        self.right = right

class Node:
    def __init__(self, val=0, neighbors=None):
        self.val = val
        self.neighbors = neighbors if neighbors is not None else []

"#;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRequest {
    pub language: String,
    pub source: String,
    pub entry: String,
    pub mode: String,
    pub helpers: Vec<String>,
    pub param_names: Vec<String>,
    pub tests: Vec<TestCase>,
    pub python_path: Option<String>,
    pub node_path: Option<String>,
    #[serde(default)]
    pub submit: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestCase {
    pub args: Option<Value>,
    pub expected: Value,
    #[serde(default)]
    pub compare: Option<String>,
    #[serde(default)]
    pub extra: Option<Value>,
    #[serde(default)]
    pub ops: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseResult {
    pub index: usize,
    pub passed: bool,
    #[serde(default)]
    pub expected: Value,
    #[serde(default)]
    pub actual: Value,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub stdout: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JudgeOutput {
    pub verdict: String,
    pub passed: usize,
    pub total: usize,
    pub stdout: String,
    pub stderr: String,
    pub cases: Vec<CaseResult>,
}

#[derive(Debug, thiserror::Error)]
pub enum JudgeError {
    #[error("{0}")]
    Message(String),
}

impl From<std::io::Error> for JudgeError {
    fn from(value: std::io::Error) -> Self {
        JudgeError::Message(value.to_string())
    }
}

impl From<serde_json::Error> for JudgeError {
    fn from(value: serde_json::Error) -> Self {
        JudgeError::Message(value.to_string())
    }
}

pub fn detect_python(override_path: Option<&str>) -> Option<String> {
    if let Some(p) = override_path {
        if !p.is_empty() && Path::new(p).exists() {
            return Some(p.to_string());
        }
    }
    which::which("python3")
        .or_else(|_| which::which("python"))
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}

pub fn detect_node(override_path: Option<&str>) -> Option<String> {
    if let Some(p) = override_path {
        if !p.is_empty() && Path::new(p).exists() {
            return Some(p.to_string());
        }
    }
    which::which("node")
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}

pub fn run(req: RunRequest) -> Result<JudgeOutput, JudgeError> {
    let timeout = if req.submit {
        SUBMIT_TIMEOUT
    } else {
        RUN_TIMEOUT
    };
    run_with_timeout(req, timeout)
}

pub fn run_with_timeout(req: RunRequest, timeout: Duration) -> Result<JudgeOutput, JudgeError> {
    let dir = tempfile_dir()?;
    let tests_json = serde_json::to_string(&req.tests)?;
    let helpers_json = serde_json::to_string(&req.helpers)?;
    let params_json = serde_json::to_string(&req.param_names)?;
    let result_path = dir.join("results.json");
    let tests_path = dir.join("tests.json");
    fs::write(&tests_path, tests_json)?;

    let (bin, args) = match req.language.as_str() {
        "python" => {
            let py = detect_python(req.python_path.as_deref()).ok_or_else(|| {
                JudgeError::Message("Python 3 was not found. Set a path in Settings.".into())
            })?;
            fs::write(
                dir.join("solution.py"),
                format!("{}{}", PYTHON_PRELUDE, req.source),
            )?;
            fs::write(dir.join("harness.py"), python_harness())?;
            (
                py,
                vec![
                    dir.join("harness.py").to_string_lossy().to_string(),
                    dir.join("solution.py").to_string_lossy().to_string(),
                    tests_path.to_string_lossy().to_string(),
                    result_path.to_string_lossy().to_string(),
                    req.entry.clone(),
                    req.mode.clone(),
                    helpers_json,
                    params_json,
                ],
            )
        }
        "javascript" => {
            let node = detect_node(req.node_path.as_deref()).ok_or_else(|| {
                JudgeError::Message("Node.js was not found. Set a path in Settings.".into())
            })?;
            fs::write(dir.join("solution.js"), &req.source)?;
            fs::write(dir.join("harness.js"), javascript_harness())?;
            (
                node,
                vec![
                    dir.join("harness.js").to_string_lossy().to_string(),
                    dir.join("solution.js").to_string_lossy().to_string(),
                    tests_path.to_string_lossy().to_string(),
                    result_path.to_string_lossy().to_string(),
                    req.entry.clone(),
                    req.mode.clone(),
                    helpers_json,
                    params_json,
                ],
            )
        }
        other => {
            let _ = fs::remove_dir_all(&dir);
            return Err(JudgeError::Message(format!("Unsupported language: {other}")));
        }
    };

    let stdout_path = dir.join("stdout.txt");
    let stderr_path = dir.join("stderr.txt");
    let stdout_file = File::create(&stdout_path)?;
    let stderr_file = File::create(&stderr_path)?;

    let mut cmd = Command::new(&bin);
    cmd.args(&args)
        .current_dir(&dir)
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let mut child = cmd.spawn()?;

    let timed_out = match child.wait_timeout(timeout)? {
        Some(_) => false,
        None => {
            kill_judge_process(&mut child);
            true
        }
    };

    let stdout = fs::read_to_string(&stdout_path).unwrap_or_default();
    let stderr = fs::read_to_string(&stderr_path).unwrap_or_default();
    let parsed = fs::read_to_string(&result_path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok());

    let _ = fs::remove_dir_all(&dir);

    if timed_out {
        return Ok(JudgeOutput {
            verdict: "tle".into(),
            passed: 0,
            total: req.tests.len(),
            stdout,
            stderr: format!("Time limit exceeded ({}s).", timeout.as_secs()),
            cases: vec![],
        });
    }

    let Some(payload) = parsed else {
        return Ok(JudgeOutput {
            verdict: "runtime_error".into(),
            passed: 0,
            total: req.tests.len(),
            stdout,
            stderr: if stderr.trim().is_empty() {
                "Runner produced no results. Check your syntax.".into()
            } else {
                stderr
            },
            cases: vec![],
        });
    };

    if let Some(err) = payload.get("compileError").and_then(|v| v.as_str()) {
        return Ok(JudgeOutput {
            verdict: "runtime_error".into(),
            passed: 0,
            total: req.tests.len(),
            stdout,
            stderr: err.to_string(),
            cases: vec![],
        });
    }

    let cases = parse_cases(&payload);
    let passed = cases.iter().filter(|c| c.passed).count();
    let total = if cases.is_empty() {
        req.tests.len()
    } else {
        cases.len()
    };

    let stdout = if stdout.trim().is_empty() {
        cases
            .iter()
            .filter_map(|c| c.stdout.as_deref())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        stdout
    };

    let has_runtime = cases.iter().any(|c| {
        c.error.as_ref().is_some_and(|e| {
            !e.is_empty()
                && e != "runtime"
                && !e.contains("did not return a value")
        }) && !c.passed
    });
    let verdict = if total == 0 {
        "runtime_error"
    } else if passed == total {
        "accepted"
    } else if has_runtime && passed == 0 {
        "runtime_error"
    } else {
        "wrong_answer"
    };

    Ok(JudgeOutput {
        verdict: verdict.into(),
        passed,
        total,
        stdout,
        stderr,
        cases,
    })
}

fn parse_cases(payload: &Value) -> Vec<CaseResult> {
    let Some(arr) = payload.get("cases").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .enumerate()
        .map(|(i, v)| {
            let actual = v.get("actual").cloned().unwrap_or(Value::Null);
            let expected = v.get("expected").cloned().unwrap_or(Value::Null);
            let mut error = match v.get("error") {
                None | Some(Value::Null) => None,
                Some(Value::String(s)) if s.is_empty() => None,
                Some(Value::String(s)) => Some(s.clone()),
                Some(other) => Some(other.to_string()),
            };
            let passed = v.get("passed").and_then(|x| x.as_bool()).unwrap_or(false);
            if !passed && actual.is_null() && error.is_none() {
                error = Some(
                    "Your function did not return a value. Add a return statement.".into(),
                );
            }
            CaseResult {
                index: v.get("index")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(i as u64) as usize,
                passed,
                expected,
                actual,
                error,
                stdout: match v.get("stdout") {
                    None | Some(Value::Null) => None,
                    Some(Value::String(s)) => Some(s.clone()),
                    Some(other) => Some(other.to_string()),
                },
            }
        })
        .collect()
}

fn tempfile_dir() -> Result<PathBuf, JudgeError> {
    let base = std::env::temp_dir().join("learndsa");
    fs::create_dir_all(&base)?;
    let dir = base.join(uuid::Uuid::new_v4().to_string());
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn kill_judge_process(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        let pid = child.id() as i32;
        unsafe {
            libc::killpg(pid, libc::SIGKILL);
        }
        let _ = child.wait();
    }
    #[cfg(windows)]
    {
        let pid = child.id().to_string();
        let _ = Command::new("taskkill")
            .args(["/PID", &pid, "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = child.wait();
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn python_harness() -> &'static str {
    include_str!("harness.py")
}

fn javascript_harness() -> &'static str {
    include_str!("harness.js")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog;
    use std::path::PathBuf;

    fn content_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../content")
    }

    fn two_sum_req(language: &str, source: &str, submit: bool) -> RunRequest {
        let problem = catalog::load_problem(&content_dir(), "two-sum").unwrap();
        let tests = if submit {
            let mut t = problem.tests.visible.clone();
            t.extend(problem.tests.hidden);
            t
        } else {
            problem.tests.visible
        };
        let entry = if language == "python" {
            problem.entry.python
        } else {
            problem.entry.javascript
        };
        RunRequest {
            language: language.into(),
            source: source.into(),
            entry,
            mode: problem.mode,
            helpers: problem.helpers,
            param_names: problem.param_names,
            tests,
            python_path: None,
            node_path: None,
            submit,
        }
    }

    fn py_available() -> bool {
        detect_python(None).is_some()
    }

    fn node_available() -> bool {
        detect_node(None).is_some()
    }

    const PY_OK: &str = r#"
class Solution:
    def twoSum(self, nums, target):
        seen = {}
        for i, n in enumerate(nums):
            if target - n in seen:
                return [seen[target - n], i]
            seen[n] = i
"#;

    const JS_OK: &str = r#"
var twoSum = function(nums, target) {
    const seen = new Map();
    for (let i = 0; i < nums.length; i++) {
        if (seen.has(target - nums[i])) return [seen.get(target - nums[i]), i];
        seen.set(nums[i], i);
    }
};
"#;

    #[test]
    fn python_two_sum_accepted() {
        if !py_available() {
            return;
        }
        let out = run(two_sum_req("python", PY_OK, true)).unwrap();
        assert_eq!(out.verdict, "accepted");
        assert_eq!(out.passed, out.total);
        assert!(out.total >= 4);
    }

    #[test]
    fn javascript_two_sum_accepted() {
        if !node_available() {
            return;
        }
        let out = run(two_sum_req("javascript", JS_OK, true)).unwrap();
        assert_eq!(out.verdict, "accepted");
        assert_eq!(out.passed, out.total);
    }

    #[test]
    fn python_missing_return() {
        if !py_available() {
            return;
        }
        let src = "class Solution:\n    def twoSum(self, nums, target):\n        pass\n";
        let out = run(two_sum_req("python", src, false)).unwrap();
        assert_ne!(out.verdict, "accepted");
        assert!(out.cases.iter().any(|c| !c.passed));
    }

    #[test]
    fn python_compile_error() {
        if !py_available() {
            return;
        }
        let out = run(two_sum_req("python", "def (\n", false)).unwrap();
        assert_eq!(out.verdict, "runtime_error");
        assert!(out.stderr.contains("SyntaxError") || out.stderr.contains("syntax"));
    }

    #[test]
    fn python_tle() {
        if !py_available() {
            return;
        }
        let src = "class Solution:\n    def twoSum(self, nums, target):\n        while True:\n            pass\n";
        let out = run_with_timeout(two_sum_req("python", src, false), Duration::from_millis(800)).unwrap();
        assert_eq!(out.verdict, "tle");
    }

    #[test]
    fn first_bad_version_python() {
        if !py_available() {
            return;
        }
        let problem = catalog::load_problem(&content_dir(), "first-bad-version").unwrap();
        let src = r#"
class Solution:
    def firstBadVersion(self, n):
        lo, hi = 1, n
        while lo < hi:
            mid = (lo + hi) // 2
            if isBadVersion(mid):
                hi = mid
            else:
                lo = mid + 1
        return lo
"#;
        let req = RunRequest {
            language: "python".into(),
            source: src.into(),
            entry: problem.entry.python,
            mode: problem.mode,
            helpers: problem.helpers,
            param_names: problem.param_names,
            tests: problem.tests.visible,
            python_path: None,
            node_path: None,
            submit: false,
        };
        let out = run(req).unwrap();
        assert_eq!(out.verdict, "accepted");
    }

    #[test]
    fn clone_graph_python() {
        if !py_available() {
            return;
        }
        let problem = catalog::load_problem(&content_dir(), "clone-graph").unwrap();
        let src = r#"
class Solution:
    def cloneGraph(self, node):
        if not node:
            return None
        copies = {}
        def dfs(n):
            if n.val in copies:
                return copies[n.val]
            c = Node(n.val)
            copies[n.val] = c
            c.neighbors = [dfs(x) for x in n.neighbors]
            return c
        return dfs(node)
"#;
        let req = RunRequest {
            language: "python".into(),
            source: src.into(),
            entry: problem.entry.python,
            mode: problem.mode,
            helpers: problem.helpers,
            param_names: problem.param_names,
            tests: problem.tests.visible,
            python_path: None,
            node_path: None,
            submit: false,
        };
        let out = run(req).unwrap();
        assert_eq!(out.verdict, "accepted", "{}", out.stderr);
    }

    #[test]
    fn lru_cache_python() {
        if !py_available() {
            return;
        }
        let problem = catalog::load_problem(&content_dir(), "lru-cache").unwrap();
        let src = r#"
class LRUCache:
    def __init__(self, capacity):
        self.cap = capacity
        self.d = {}
    def get(self, key):
        if key not in self.d:
            return -1
        v = self.d.pop(key)
        self.d[key] = v
        return v
    def put(self, key, value):
        if key in self.d:
            self.d.pop(key)
        self.d[key] = value
        if len(self.d) > self.cap:
            self.d.pop(next(iter(self.d)))
"#;
        let mut tests = problem.tests.visible.clone();
        tests.extend(problem.tests.hidden);
        let req = RunRequest {
            language: "python".into(),
            source: src.into(),
            entry: problem.entry.python,
            mode: problem.mode,
            helpers: problem.helpers,
            param_names: problem.param_names,
            tests,
            python_path: None,
            node_path: None,
            submit: true,
        };
        let out = run(req).unwrap();
        assert_eq!(out.verdict, "accepted", "{}", out.stderr);
    }
}
