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
pub struct CppTypes {
    #[serde(rename = "return")]
    #[allow(dead_code)]
    pub return_type: String,
    pub params: Vec<String>,
}

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
    pub cpp_path: Option<String>,
    #[serde(default)]
    pub cpp_types: Option<CppTypes>,
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

pub fn detect_cpp(override_path: Option<&str>) -> Option<String> {
    if let Some(p) = override_path {
        if !p.is_empty() && Path::new(p).exists() {
            return Some(p.to_string());
        }
    }
    which::which("g++")
        .or_else(|_| which::which("clang++"))
        .or_else(|_| which::which("c++"))
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
        "cpp" => {
            let cxx = detect_cpp(req.cpp_path.as_deref()).ok_or_else(|| {
                JudgeError::Message(
                    "A C++ compiler (g++/clang++) was not found. Set a path in Settings.".into(),
                )
            })?;
            let out_kind = req.helpers.first().cloned().unwrap_or_else(|| "json".into());
            let invoke = generate_cpp_invoke(&req, &out_kind)?;
            fs::write(dir.join("solution.cpp"), &req.source)?;
            fs::write(dir.join("invoke.inc"), invoke)?;
            fs::write(dir.join("harness.cpp"), cpp_harness())?;
            let runner = dir.join(if cfg!(windows) { "runner.exe" } else { "runner" });
            let compile = Command::new(&cxx)
                .args([
                    "-std=c++17",
                    "-O2",
                    "-pipe",
                    "-o",
                    runner.to_str().unwrap_or("runner"),
                    "harness.cpp",
                ])
                .current_dir(&dir)
                .output()?;
            if !compile.status.success() {
                let mut err = String::from_utf8_lossy(&compile.stderr).to_string();
                if err.trim().is_empty() {
                    err = String::from_utf8_lossy(&compile.stdout).to_string();
                }
                let _ = fs::remove_dir_all(&dir);
                return Ok(JudgeOutput {
                    verdict: "runtime_error".into(),
                    passed: 0,
                    total: req.tests.len(),
                    stdout: String::new(),
                    stderr: if err.trim().is_empty() {
                        "C++ compile failed.".into()
                    } else {
                        err
                    },
                    cases: vec![],
                });
            }
            (
                runner.to_string_lossy().to_string(),
                vec![
                    dir.join("solution.cpp").to_string_lossy().to_string(),
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

fn cpp_harness() -> &'static str {
    include_str!("harness.cpp")
}

fn strip_ref(ty: &str) -> &str {
    ty.strip_suffix('&').unwrap_or(ty).trim()
}

fn cpp_decode_expr(ty: &str, name: &str) -> Result<String, JudgeError> {
    let base = strip_ref(ty);
    let expr = match base {
        "int" => format!("args[\"{name}\"].as_int()"),
        "bool" => format!("args[\"{name}\"].as_bool()"),
        "string" => format!("args[\"{name}\"].as_string()"),
        "vector<int>" => format!("as_vector_int(args[\"{name}\"])"),
        "vector<string>" => format!("as_vector_string(args[\"{name}\"])"),
        "vector<vector<int>>" => format!("as_vector_vector_int(args[\"{name}\"])"),
        "vector<vector<char>>" => format!("as_vector_vector_char(args[\"{name}\"])"),
        "vector<ListNode*>" => format!("decode_listlist(args[\"{name}\"])"),
        "ListNode*" => format!("decode_list(args[\"{name}\"], extra)"),
        "TreeNode*" => format!("decode_tree(args[\"{name}\"])"),
        "Node*" => format!("decode_graph(args[\"{name}\"])"),
        other => {
            return Err(JudgeError::Message(format!(
                "Unsupported C++ parameter type: {other}"
            )));
        }
    };
    Ok(format!("  {base} p_{name} = {expr};"))
}

fn generate_cpp_invoke(req: &RunRequest, out_kind: &str) -> Result<String, JudgeError> {
    if req.mode == "class" {
        return Ok(cpp_class_invoke(&req.entry));
    }
    let types = req.cpp_types.as_ref().ok_or_else(|| {
        JudgeError::Message("Missing C++ type metadata for this problem.".into())
    })?;
    if types.params.len() != req.param_names.len() {
        return Err(JudgeError::Message(
            "C++ parameter types do not match param names.".into(),
        ));
    }
    let mut lines = Vec::new();
    lines.push("static Json run_function_case(const Json& args, const Json& extra, const Json& /*params*/, const vector<string>& /*in_kinds*/, const string& out_kind) {".into());
    lines.push("  (void)out_kind;".into());
    lines.push("  Solution sol;".into());
    for (name, ty) in req.param_names.iter().zip(types.params.iter()) {
        lines.push(cpp_decode_expr(ty, name)?);
    }
    let args_list = req
        .param_names
        .iter()
        .map(|n| format!("p_{n}"))
        .collect::<Vec<_>>()
        .join(", ");
    lines.push(format!(
        "  auto raw = sol.{}({});",
        req.entry, args_list
    ));
    lines.push(format!(
        "  return encode_by_kind(raw, \"{}\");",
        out_kind.replace('"', "")
    ));
    lines.push("}".into());
    lines.push(String::new());
    lines.push("static Json run_class_case(const string&, const Json&) {".into());
    lines.push("  throw runtime_error(\"not a class problem\");".into());
    lines.push("}".into());
    Ok(lines.join("\n"))
}

fn cpp_class_invoke(entry: &str) -> String {
    let body = match entry {
        "LRUCache" => r#"
static Json run_class_case(const string& entry, const Json& ops) {
  (void)entry;
  vector<Json> actual;
  unique_ptr<LRUCache> obj;
  for (const auto& step : ops.as_array()) {
    const auto& arr = step.as_array();
    string name = arr[0].as_string();
    if (!obj) {
      obj.reset(new LRUCache(arr[1].as_int()));
      actual.push_back(Json::null());
    } else if (name == "get") {
      actual.push_back(Json::number(obj->get(arr[1].as_int())));
    } else if (name == "put") {
      obj->put(arr[1].as_int(), arr[2].as_int());
      actual.push_back(Json::null());
    } else {
      throw runtime_error("unknown LRUCache method: " + name);
    }
  }
  return Json::array(actual);
}
"#,
        "Trie" => r#"
static Json run_class_case(const string& entry, const Json& ops) {
  (void)entry;
  vector<Json> actual;
  unique_ptr<Trie> obj;
  for (const auto& step : ops.as_array()) {
    const auto& arr = step.as_array();
    string name = arr[0].as_string();
    if (!obj) {
      obj.reset(new Trie());
      actual.push_back(Json::null());
    } else if (name == "insert") {
      obj->insert(arr[1].as_string());
      actual.push_back(Json::null());
    } else if (name == "search") {
      actual.push_back(Json::boolean(obj->search(arr[1].as_string())));
    } else if (name == "startsWith") {
      actual.push_back(Json::boolean(obj->startsWith(arr[1].as_string())));
    } else {
      throw runtime_error("unknown Trie method: " + name);
    }
  }
  return Json::array(actual);
}
"#,
        _ => r#"
static Json run_class_case(const string& entry, const Json&) {
  throw runtime_error("unsupported C++ class entry: " + entry);
}
"#,
    };
    format!(
        "{body}\nstatic Json run_function_case(const Json&, const Json&, const Json&, const vector<string>&, const string&) {{\n  throw runtime_error(\"not a function problem\");\n}}\n"
    )
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
        let entry = match language {
            "python" => problem.entry.python,
            "javascript" => problem.entry.javascript,
            "cpp" => problem.entry.cpp.unwrap_or(problem.entry.python),
            _ => problem.entry.python,
        };
        let cpp_types = problem.cpp_types.map(|t| CppTypes {
            return_type: t.return_type,
            params: t.params,
        });
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
            cpp_path: None,
            cpp_types,
            submit,
        }
    }

    fn py_available() -> bool {
        detect_python(None).is_some()
    }

    fn node_available() -> bool {
        detect_node(None).is_some()
    }

    fn cpp_available() -> bool {
        detect_cpp(None).is_some()
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

    const CPP_OK: &str = r#"
class Solution {
public:
    vector<int> twoSum(vector<int>& nums, int target) {
        unordered_map<int, int> seen;
        for (int i = 0; i < (int)nums.size(); i++) {
            auto it = seen.find(target - nums[i]);
            if (it != seen.end()) return {it->second, i};
            seen[nums[i]] = i;
        }
        return {};
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
    fn cpp_two_sum_accepted() {
        if !cpp_available() {
            return;
        }
        let out = run(two_sum_req("cpp", CPP_OK, true)).unwrap();
        assert_eq!(out.verdict, "accepted", "{}", out.stderr);
        assert_eq!(out.passed, out.total);
    }

    #[test]
    fn cpp_reverse_list_accepted() {
        if !cpp_available() {
            return;
        }
        let problem = catalog::load_problem(&content_dir(), "reverse-linked-list").unwrap();
        let src = r#"
class Solution {
public:
    ListNode* reverseList(ListNode* head) {
        ListNode* prev = nullptr;
        while (head) {
            ListNode* nxt = head->next;
            head->next = prev;
            prev = head;
            head = nxt;
        }
        return prev;
    }
};
"#;
        let req = RunRequest {
            language: "cpp".into(),
            source: src.into(),
            entry: problem.entry.cpp.unwrap(),
            mode: problem.mode,
            helpers: problem.helpers,
            param_names: problem.param_names,
            tests: problem.tests.visible,
            python_path: None,
            node_path: None,
            cpp_path: None,
            cpp_types: problem.cpp_types.map(|t| CppTypes {
                return_type: t.return_type,
                params: t.params,
            }),
            submit: false,
        };
        let out = run(req).unwrap();
        assert_eq!(out.verdict, "accepted", "{}", out.stderr);
    }

    #[test]
    fn cpp_lru_cache_accepted() {
        if !cpp_available() {
            return;
        }
        let problem = catalog::load_problem(&content_dir(), "lru-cache").unwrap();
        let src = r#"
class LRUCache {
    int cap;
    list<pair<int,int>> order;
    unordered_map<int, list<pair<int,int>>::iterator> pos;
public:
    LRUCache(int capacity) : cap(capacity) {}
    int get(int key) {
        auto it = pos.find(key);
        if (it == pos.end()) return -1;
        order.splice(order.begin(), order, it->second);
        return it->second->second;
    }
    void put(int key, int value) {
        auto it = pos.find(key);
        if (it != pos.end()) {
            it->second->second = value;
            order.splice(order.begin(), order, it->second);
            return;
        }
        if ((int)order.size() == cap) {
            pos.erase(order.back().first);
            order.pop_back();
        }
        order.emplace_front(key, value);
        pos[key] = order.begin();
    }
};
"#;
        let mut tests = problem.tests.visible.clone();
        tests.extend(problem.tests.hidden);
        let req = RunRequest {
            language: "cpp".into(),
            source: src.into(),
            entry: problem.entry.cpp.unwrap(),
            mode: problem.mode,
            helpers: problem.helpers,
            param_names: problem.param_names,
            tests,
            python_path: None,
            node_path: None,
            cpp_path: None,
            cpp_types: None,
            submit: true,
        };
        let out = run(req).unwrap();
        assert_eq!(out.verdict, "accepted", "{}", out.stderr);
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
            cpp_path: None,
            cpp_types: None,
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
            cpp_path: None,
            cpp_types: None,
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
            cpp_path: None,
            cpp_types: None,
            submit: true,
        };
        let out = run(req).unwrap();
        assert_eq!(out.verdict, "accepted", "{}", out.stderr);
    }
}
