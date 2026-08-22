use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tauri::State;

use crate::ollama;
use crate::progress::{self, Db, Settings};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewRequest {
    pub problem_id: String,
    pub title: String,
    pub statement: String,
    pub tags: Vec<String>,
    pub language: String,
    pub code: String,
}

const SYSTEM_PROMPT: &str = "You are an interview coach reviewing a solution the student already got Accepted on. \
Cover: (1) time and space complexity, (2) edge cases they should mention in an interview, (3) a cleaner or more idiomatic approach if one exists. \
Do not provide a full rewritten solution or a complete drop-in replacement. Short snippets that illustrate an idea are fine. \
Be direct and concise. Reply in markdown.";

const COMPLEXITY_SYSTEM_PROMPT: &str = "You analyze algorithm solutions and report Big-O complexity. \
Reply with JSON only, no markdown fences, no extra text. Exact shape: \
{\"timeComplexity\":\"O(...)\",\"spaceComplexity\":\"O(...)\"}. \
Use standard Big-O in terms of input size (n, m, etc. as appropriate). Be concise.";

pub async fn review_solution(db: State<'_, Db>, req: ReviewRequest) -> Result<String, String> {
    let (settings, accepted) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let settings = progress::load_settings(&conn);
        let accepted = progress::is_accepted(&conn, &req.problem_id);
        (settings, accepted)
    };

    if !accepted {
        return Err("Review unlocks after you get Accepted on this problem.".into());
    }
    ensure_ai_configured(&settings)?;

    let user = format!(
        "Problem: {title}\nTags: {tags}\nLanguage: {language}\n\nStatement:\n{statement}\n\nCode:\n```{language}\n{code}\n```",
        title = req.title,
        tags = req.tags.join(", "),
        language = req.language,
        statement = req.statement,
        code = req.code,
    );

    let text = chat_completion(&settings, SYSTEM_PROMPT, &user, false).await?;

    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        progress::save_ai_review(&conn, &req.problem_id, &req.language, &text)
            .map_err(|e| e.to_string())?;
    }

    Ok(text)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplexityResult {
    pub attempt_id: String,
    pub time_complexity: String,
    pub space_complexity: String,
}

pub async fn analyze_attempt_complexity(
    db: State<'_, Db>,
    attempt_id: String,
    force: bool,
) -> Result<ComplexityResult, String> {
    let (settings, cached, attempt) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let settings = progress::load_settings(&conn);
        let cached = progress::load_attempt_complexity(&conn, &attempt_id);
        let attempt = progress::get_attempt(&conn, &attempt_id);
        (settings, cached, attempt)
    };

    if !force {
        if let Some(c) = cached {
            return Ok(ComplexityResult {
                attempt_id: c.attempt_id,
                time_complexity: c.time_complexity,
                space_complexity: c.space_complexity,
            });
        }
    }

    let attempt = attempt.ok_or_else(|| "Submission not found.".to_string())?;
    ensure_ai_configured(&settings)?;

    let user = format!(
        "Language: {language}\n\nCode:\n```{language}\n{code}\n```",
        language = attempt.language,
        code = attempt.code,
    );

    let raw = chat_completion(&settings, COMPLEXITY_SYSTEM_PROMPT, &user, true).await?;
    let (time_complexity, space_complexity) = parse_complexity_json(&raw)?;

    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        progress::save_attempt_complexity(&conn, &attempt_id, &time_complexity, &space_complexity)
            .map_err(|e| e.to_string())?;
    }

    Ok(ComplexityResult {
        attempt_id,
        time_complexity,
        space_complexity,
    })
}

fn ensure_ai_configured(settings: &Settings) -> Result<(), String> {
    let provider = settings.ai_provider.trim().to_lowercase();
    if provider == "ollama" {
        if settings.ollama_model.trim().is_empty() {
            return Err("Select or install an Ollama model in Settings.".into());
        }
    } else if settings.openai_api_key.trim().is_empty() {
        return Err("Add an OpenAI API key in Settings.".into());
    }
    Ok(())
}

fn extract_json_object(raw: &str) -> Result<&str, String> {
    let trimmed = raw.trim();
    let start = trimmed.find('{').ok_or_else(|| "AI returned no JSON object.".to_string())?;
    let bytes = trimmed.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_str {
            if escape {
                escape = false;
                continue;
            }
            if b == b'\\' {
                escape = true;
                continue;
            }
            if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(&trimmed[start..=i]);
                }
            }
            _ => {}
        }
    }
    Err("AI returned no JSON object.".into())
}

fn plausible_big_o(s: &str) -> bool {
    let s = s.trim();
    if s.len() < 3 || s.len() > 48 || s.contains('\n') {
        return false;
    }
    let open = s.find('(');
    let close = s.rfind(')');
    matches!((open, close), (Some(o), Some(c)) if o < c && c == s.len() - 1)
}

pub(crate) fn parse_complexity_json(raw: &str) -> Result<(String, String), String> {
    let json_str = extract_json_object(raw)?;
    let value: Value =
        serde_json::from_str(json_str).map_err(|e| format!("Could not parse complexity JSON: {e}"))?;

    let time = value
        .get("timeComplexity")
        .or_else(|| value.get("time_complexity"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| plausible_big_o(s))
        .ok_or_else(|| "AI response missing a valid timeComplexity.".to_string())?;
    let space = value
        .get("spaceComplexity")
        .or_else(|| value.get("space_complexity"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| plausible_big_o(s))
        .ok_or_else(|| "AI response missing a valid spaceComplexity.".to_string())?;

    Ok((time.to_string(), space.to_string()))
}

async fn chat_completion(
    settings: &Settings,
    system: &str,
    user: &str,
    json_object: bool,
) -> Result<String, String> {
    let provider = settings.ai_provider.trim().to_lowercase();
    match provider.as_str() {
        "ollama" => {
            chat_with_ollama(
                &settings.ollama_host,
                &settings.ollama_model,
                system,
                user,
                json_object,
            )
            .await
        }
        _ => chat_with_openai(&settings.openai_api_key, system, user, json_object).await,
    }
}

async fn chat_with_openai(
    api_key: &str,
    system: &str,
    user: &str,
    json_object: bool,
) -> Result<String, String> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err("Add an OpenAI API key in Settings.".into());
    }

    let client = ollama::no_redirect_client(Some(Duration::from_secs(30)))?;

    let mut body = json!({
        "model": "gpt-4o-mini",
        "temperature": 0.2,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ]
    });
    if json_object {
        body["response_format"] = json!({ "type": "json_object" });
    }

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Could not reach OpenAI: {e}"))?;

    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("OpenAI returned an unreadable response: {e}"))?;

    if status.as_u16() == 401 {
        return Err("Invalid OpenAI API key. Check Settings.".into());
    }
    if !status.is_success() {
        let msg = body
            .pointer("/error/message")
            .and_then(|v| v.as_str())
            .unwrap_or("OpenAI request failed.");
        return Err(msg.to_string());
    }

    body.pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "OpenAI returned an empty response.".into())
}

async fn chat_with_ollama(
    host: &str,
    model: &str,
    system: &str,
    user: &str,
    json_object: bool,
) -> Result<String, String> {
    let host = ollama::validate_host(host)?;
    let model = model.trim();
    if model.is_empty() {
        return Err("Select or install an Ollama model in Settings.".into());
    }

    let client = ollama::no_redirect_client(Some(Duration::from_secs(180)))?;

    let mut body = json!({
        "model": model,
        "temperature": 0.2,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ]
    });
    if json_object {
        body["response_format"] = json!({ "type": "json_object" });
    }

    let url = format!("{host}/v1/chat/completions");
    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Could not reach Ollama. Is it running? ({e})"))?;

    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("Ollama returned an unreadable response: {e}"))?;

    if !status.is_success() {
        let msg = body
            .pointer("/error/message")
            .or_else(|| body.get("error"))
            .and_then(|v| v.as_str())
            .unwrap_or("Ollama request failed.");
        return Err(msg.to_string());
    }

    body.pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Ollama returned an empty response.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_camel_case_json() {
        let (t, s) = parse_complexity_json(
            r#"{"timeComplexity":"O(n)","spaceComplexity":"O(1)"}"#,
        )
        .unwrap();
        assert_eq!(t, "O(n)");
        assert_eq!(s, "O(1)");
    }

    #[test]
    fn parse_snake_case_and_fences() {
        let raw = "```json\n{\"time_complexity\":\"O(n log n)\",\"space_complexity\":\"O(n)\"}\n```";
        let (t, s) = parse_complexity_json(raw).unwrap();
        assert_eq!(t, "O(n log n)");
        assert_eq!(s, "O(n)");
    }

    #[test]
    fn parse_ignores_trailing_prose_object() {
        let raw = r#"{"timeComplexity":"O(n)","spaceComplexity":"O(1)"} extra {"nope":true}"#;
        let (t, s) = parse_complexity_json(raw).unwrap();
        assert_eq!(t, "O(n)");
        assert_eq!(s, "O(1)");
    }

    #[test]
    fn parse_rejects_missing_keys() {
        assert!(parse_complexity_json(r#"{"timeComplexity":"O(n)"}"#).is_err());
    }

    #[test]
    fn parse_rejects_empty_or_paragraph() {
        assert!(parse_complexity_json(
            r#"{"timeComplexity":"","spaceComplexity":"O(1)"}"#
        )
        .is_err());
        assert!(parse_complexity_json(
            r#"{"timeComplexity":"this is a long essay about the algorithm that is not big-o","spaceComplexity":"O(1)"}"#
        )
        .is_err());
        assert!(parse_complexity_json("no json here").is_err());
    }
}
