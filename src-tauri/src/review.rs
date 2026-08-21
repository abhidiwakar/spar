use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
use tauri::State;

use crate::progress::{self, Db};

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

pub async fn review_solution(db: State<'_, Db>, req: ReviewRequest) -> Result<String, String> {
    let (api_key, accepted) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let key = progress::load_settings(&conn).openai_api_key;
        let accepted = progress::is_accepted(&conn, &req.problem_id);
        (key, accepted)
    };

    if !accepted {
        return Err("Review unlocks after you get Accepted on this problem.".into());
    }
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err("Add an OpenAI API key in Settings.".into());
    }

    let user = format!(
        "Problem: {title}\nTags: {tags}\nLanguage: {language}\n\nStatement:\n{statement}\n\nCode:\n```{language}\n{code}\n```",
        title = req.title,
        tags = req.tags.join(", "),
        language = req.language,
        statement = req.statement,
        code = req.code,
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&json!({
            "model": "gpt-4o-mini",
            "temperature": 0.3,
            "messages": [
                { "role": "system", "content": SYSTEM_PROMPT },
                { "role": "user", "content": user }
            ]
        }))
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
        .ok_or_else(|| "OpenAI returned an empty review.".into())
}
