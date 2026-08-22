use futures_util::StreamExt;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tauri::ipc::Channel;

/// Incremented for each pull. Cancel records the generation that should stop.
static PULL_GENERATION: AtomicU64 = AtomicU64::new(0);
static CANCELLED_GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaStatus {
    pub running: bool,
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaPullProgress {
    pub status: String,
    pub completed: Option<u64>,
    pub total: Option<u64>,
}

fn json_u64(v: &Value) -> Option<u64> {
    v.as_u64()
        .or_else(|| v.as_i64().and_then(|n| u64::try_from(n).ok()))
        .or_else(|| v.as_f64().and_then(|n| (n.is_finite() && n >= 0.0).then_some(n as u64)))
}

fn is_loopback_host(host: &str) -> bool {
    let host = host.trim_matches(|c| c == '[' || c == ']');
    host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1"
}

fn origin_from_url(parsed: &Url) -> String {
    let host = parsed.host_str().unwrap_or("127.0.0.1");
    let host = if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    match parsed.port() {
        Some(port) => format!("{}://{host}:{port}", parsed.scheme()),
        None => format!("{}://{host}", parsed.scheme()),
    }
}

/// Only allow loopback origins so a user-edited host cannot be used for SSRF.
pub fn validate_host(host: &str) -> Result<String, String> {
    let raw = host.trim();
    if raw.is_empty() {
        return Err("Ollama host is empty.".into());
    }
    let parsed = Url::parse(raw).map_err(|_| "Ollama host is not a valid URL.".to_string())?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("Ollama host must use http or https.".into());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("Ollama host must be http://127.0.0.1 or http://localhost.".into());
    }
    let hostname = parsed.host_str().unwrap_or("");
    if !is_loopback_host(hostname) {
        return Err("Ollama host must be http://127.0.0.1 or http://localhost.".into());
    }
    let path = parsed.path();
    if path != "/" && !path.is_empty() {
        return Err("Ollama host must not include a path.".into());
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err("Ollama host must not include a query or fragment.".into());
    }
    Ok(origin_from_url(&parsed))
}

pub fn no_redirect_client(timeout: Option<Duration>) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10));
    if let Some(timeout) = timeout {
        builder = builder.timeout(timeout);
    }
    builder.build().map_err(|e| e.to_string())
}

fn pull_cancelled(generation: u64) -> bool {
    CANCELLED_GENERATION.load(Ordering::SeqCst) >= generation
}

pub fn cancel_pull() {
    CANCELLED_GENERATION.store(PULL_GENERATION.load(Ordering::SeqCst), Ordering::SeqCst);
}

pub async fn status(host: &str) -> Result<OllamaStatus, String> {
    let host = validate_host(host)?;
    let client = no_redirect_client(Some(Duration::from_secs(3)))?;

    let url = format!("{host}/api/tags");
    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(_) => {
            return Ok(OllamaStatus {
                running: false,
                models: vec![],
            });
        }
    };

    if !response.status().is_success() {
        return Ok(OllamaStatus {
            running: false,
            models: vec![],
        });
    }

    let body: Value = match response.json().await {
        Ok(v) => v,
        Err(_) => {
            return Ok(OllamaStatus {
                running: false,
                models: vec![],
            });
        }
    };

    let models = body
        .get("models")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Ok(OllamaStatus {
        running: true,
        models,
    })
}

pub async fn pull_model(
    host: &str,
    model: &str,
    on_progress: Channel<OllamaPullProgress>,
) -> Result<(), String> {
    let host = validate_host(host)?;
    let model = model.trim();
    if model.is_empty() {
        return Err("Model name is empty.".into());
    }

    let generation = PULL_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;

    // Connect timeout only — a 7B pull can take longer than any fixed total timeout.
    let client = no_redirect_client(None)?;

    let url = format!("{host}/api/pull");
    let response = client
        .post(&url)
        .json(&json!({ "name": model, "stream": true }))
        .send()
        .await
        .map_err(|e| format!("Could not reach Ollama: {e}"))?;

    if pull_cancelled(generation) {
        return Err("Pull cancelled.".into());
    }

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Ollama pull failed ({status}): {text}"));
    }

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        if pull_cancelled(generation) {
            drop(stream);
            return Err("Pull cancelled.".into());
        }
        let chunk = chunk.map_err(|e| {
            if pull_cancelled(generation) {
                "Pull cancelled.".into()
            } else {
                format!("Pull stream error: {e}")
            }
        })?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(idx) = buffer.find('\n') {
            let line = buffer[..idx].trim().to_string();
            buffer = buffer[idx + 1..].to_string();
            if line.is_empty() {
                continue;
            }
            let parsed: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if let Some(err) = parsed.get("error").and_then(|e| e.as_str()) {
                return Err(err.to_string());
            }
            let progress = OllamaPullProgress {
                status: parsed
                    .get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
                completed: parsed.get("completed").and_then(json_u64),
                total: parsed.get("total").and_then(json_u64),
            };
            let _ = on_progress.send(progress);
        }
    }

    if pull_cancelled(generation) {
        return Err("Pull cancelled.".into());
    }

    Ok(())
}

fn path_is_safe(path: &str) -> bool {
    !path.is_empty()
        && path.starts_with('/')
        && path
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.'))
}

/// Reconstruct a https://ollama.com/... URL; reject anything else.
pub fn validate_ollama_download_url(url: &str) -> Result<String, String> {
    let err = || "Only https://ollama.com/ URLs can be opened.".to_string();
    let parsed = Url::parse(url.trim()).map_err(|_| err())?;
    if parsed.scheme() != "https" || parsed.host_str() != Some("ollama.com") {
        return Err(err());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(err());
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err(err());
    }
    if parsed.port().is_some() {
        return Err(err());
    }
    let path = parsed.path();
    if !path_is_safe(path) {
        return Err(err());
    }
    Ok(format!("https://ollama.com{path}"))
}

pub fn open_external_url(url: &str) -> Result<(), String> {
    let url = validate_ollama_download_url(url)?;

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        // Avoid cmd.exe so `&` in a URL cannot launch a second command.
        Command::new("explorer")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        return Err("Opening URLs is not supported on this platform.".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accept_loopback_hosts() {
        assert_eq!(
            validate_host("http://127.0.0.1:11434").unwrap(),
            "http://127.0.0.1:11434"
        );
        assert_eq!(
            validate_host("http://localhost:11434/").unwrap(),
            "http://localhost:11434"
        );
        assert_eq!(
            validate_host("  http://127.0.0.1  ").unwrap(),
            "http://127.0.0.1"
        );
        assert_eq!(
            validate_host("http://[::1]:11434").unwrap(),
            "http://[::1]:11434"
        );
        assert_eq!(validate_host("https://localhost").unwrap(), "https://localhost");
    }

    #[test]
    fn reject_ssrf_hosts() {
        for host in [
            "http://localhost.com",
            "http://localhost.evil.com",
            "http://127.0.0.1.attacker.com",
            "http://127.0.0.1@evil.com",
            "http://127.0.0.1.nip.io",
            "https://example.com",
            "http://8.8.8.8",
            "http://127.0.0.1/redirect",
            "http://127.0.0.1?x=1",
            "http://127.0.0.1#frag",
            "",
            "not-a-url",
        ] {
            assert!(validate_host(host).is_err(), "should reject {host}");
        }
    }

    #[test]
    fn accept_ollama_download_url() {
        assert_eq!(
            validate_ollama_download_url("https://ollama.com/download").unwrap(),
            "https://ollama.com/download"
        );
    }

    #[test]
    fn reject_dangerous_open_urls() {
        for url in [
            "https://ollama.com/download&whoami",
            "https://ollama.com.evil.com/download",
            "http://ollama.com/download",
            "https://evil.com/",
            "https://ollama.com/download?x=1",
            "https://user@ollama.com/download",
        ] {
            assert!(
                validate_ollama_download_url(url).is_err(),
                "should reject {url}"
            );
        }
    }
}
