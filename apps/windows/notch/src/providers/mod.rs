mod cloud;
mod credentials;
mod gemini;
pub mod keyring;
#[cfg(test)]
mod tests;

use crate::{usage::{LimitWindow, UsageSnapshot}, AppState};
use serde_json::Value;
use std::{collections::BTreeMap, path::Path, time::Duration};
use tauri::{AppHandle, Emitter, Manager};

pub const FAMILIES: &[&str] = &["claude", "cursor", "codex", "antigravity", "glm", "grok", "opencode", "commandcode", "copilot", "ollama-local", "ollama", "gemini-api"];
type Fetch = fn() -> Result<Vec<LimitWindow>, Failure>;
const ADAPTERS: &[(&str, Fetch)] = &[
    ("glm", cloud::glm), ("grok", cloud::grok), ("opencode", cloud::opencode),
    ("commandcode", cloud::commandcode), ("copilot", cloud::copilot),
    ("ollama", cloud::ollama), ("ollama-local", cloud::ollama_local), ("gemini-api", gemini::read),
];

#[derive(Debug)]
pub enum Failure { NeedsAuth, RateLimited(u64), Invalid(&'static str), Unavailable(String) }

pub fn now_ms() -> u64 { chrono::Utc::now().timestamp_millis().max(0) as u64 }

pub fn enabled(app: &AppHandle, id: &str) -> bool {
    !app.state::<AppState>().cfg.lock().unwrap().disabled_providers.iter().any(|disabled| disabled == id)
}

pub fn start(app: AppHandle) {
    for &(id, fetch) in ADAPTERS {
        let app = app.clone();
        std::thread::spawn(move || poll(app, id, fetch));
    }
}

fn poll(app: AppHandle, id: &str, fetch: Fetch) {
    let mut consecutive_limits = 0u32;
    loop {
        if enabled(&app, id) {
            let previous = app.state::<AppState>().providers.lock().unwrap().get(id).cloned().unwrap_or_default();
            if previous.backoff_until <= now_ms() {
                let snapshot = update_snapshot(previous, fetch(), &mut consecutive_limits);
                if enabled(&app, id) { publish(&app, id, snapshot); }
            }
        }
        std::thread::sleep(Duration::from_secs(60));
    }
}

fn update_snapshot(mut previous: UsageSnapshot, reading: Result<Vec<LimitWindow>, Failure>, attempts: &mut u32) -> UsageSnapshot {
    match reading {
        Ok(windows) => {
            *attempts = 0;
            UsageSnapshot { status: if windows.is_empty() { "none" } else { "ok" }.into(), windows, fetched_at: now_ms(), ..Default::default() }
        }
        Err(failure) => {
            let (status, note) = failure_message(failure, &mut previous, attempts);
            previous.status = status.into();
            previous.note = note;
            previous
        }
    }
}

fn failure_message(failure: Failure, snapshot: &mut UsageSnapshot, attempts: &mut u32) -> (&'static str, String) {
    match failure {
        Failure::NeedsAuth => ("needsAuth", "Sign in with the provider's own tool.".into()),
        Failure::RateLimited(retry) => {
            let seconds = (60u64 * (1u64 << (*attempts).min(4))).min(900).max(retry);
            *attempts = attempts.saturating_add(1);
            snapshot.backoff_until = now_ms().saturating_add(seconds.saturating_mul(1000));
            ("backoff", "Rate limited. Waiting before retrying.".into())
        }
        Failure::Invalid(reason) => (if snapshot.windows.is_empty() { "error" } else { "stale" }, reason.into()),
        Failure::Unavailable(reason) => (if snapshot.windows.is_empty() { "error" } else { "stale" }, reason),
    }
}

fn publish(app: &AppHandle, id: &str, snapshot: UsageSnapshot) {
    let state = app.state::<AppState>();
    let mut readings = state.providers.lock().unwrap();
    readings.insert(id.into(), snapshot);
    let path = crate::config::config_path().with_file_name("providers.json");
    match serde_json::to_vec(&*readings).map_err(|e| e.to_string()).and_then(|json| std::fs::write(path, json).map_err(|e| e.to_string())) {
        Ok(()) => {},
        Err(reason) => crate::applog(&format!("Could not save provider cache: {reason}")),
    }
    let _ = app.emit("providers", &*readings);
}

pub fn load_cache() -> BTreeMap<String, UsageSnapshot> {
    let path = crate::config::config_path().with_file_name("providers.json");
    let mut readings: BTreeMap<String, UsageSnapshot> = std::fs::read(path).ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default();
    for snapshot in readings.values_mut() {
        if !snapshot.windows.is_empty() { snapshot.status = "stale".into(); }
    }
    readings
}

fn json_file(path: &Path) -> Result<Option<Value>, Failure> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map(Some).map_err(|_| Failure::Invalid("Credential or log JSON is malformed.")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(Failure::Unavailable("Cannot read the provider's local file.".into())),
    }
}

fn home() -> Result<std::path::PathBuf, Failure> {
    dirs::home_dir().ok_or(Failure::Invalid("Home directory is unavailable."))
}

fn get(request: ureq::Request) -> Result<Value, Failure> {
    let response = request.timeout(Duration::from_secs(15)).call().map_err(|error| match error {
        ureq::Error::Status(401 | 403, _) => Failure::NeedsAuth,
        ureq::Error::Status(429, reply) => Failure::RateLimited(reply.header("Retry-After").and_then(|s| s.parse().ok()).unwrap_or(60)),
        ureq::Error::Status(_, _) => Failure::Unavailable("Provider returned an HTTP error.".into()),
        ureq::Error::Transport(_) => Failure::Unavailable("Provider could not be reached.".into()),
    })?;
    response.into_json().map_err(|_| Failure::Invalid("Provider returned malformed JSON."))
}

fn request(url: &str, token: &str) -> ureq::Request {
    // Cross-host redirects must never forward borrowed credentials.
    ureq::AgentBuilder::new().redirects(0).build().get(url)
        .set("Authorization", &format!("Bearer {token}")).set("Accept", "application/json").set("User-Agent", "Notch")
}

fn reset(stamp: &Value) -> Option<u64> {
    if let Some(text) = stamp.as_str() {
        return chrono::DateTime::parse_from_rfc3339(text).ok().map(|date| date.timestamp_millis().max(0) as u64);
    }
    let number = stamp.as_u64().filter(|number| *number > 0)?;
    Some(if number < 1_000_000_000_000 { number.saturating_mul(1000) } else { number })
}

fn window(id: &str, fraction: f64, reset_at: Option<u64>) -> LimitWindow {
    LimitWindow { id: id.into(), label: id.replace('_', " "), used: fraction.max(0.0), resets_at: reset_at, ..Default::default() }
}

fn count_window(id: &str, count: i64) -> LimitWindow {
    LimitWindow { id: id.into(), label: id.into(), count: Some(count), ..Default::default() }
}
