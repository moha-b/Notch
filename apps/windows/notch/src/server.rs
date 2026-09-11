//! Local event server: receives notch-hook's POST /event?e=<event>&ppid=<pid>[&profile=<id>]
//! with the Claude Code hook's stdin JSON as the body. Lenient parsing: no missing field is an error.

use crate::state::HookEvent;
use crate::AppState;
use std::io::Read;
use tauri::{AppHandle, Manager};

pub fn start(app: AppHandle, port: u16) {
    std::thread::spawn(move || {
        let server = match tiny_http::Server::http(("127.0.0.1", port)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[notch] failed to bind port {port}: {e} (is another instance running?)");
                return;
            }
        };
        // Profiles are discovered at launch; one created later reports as the default until Notch relaunches
        let claude_profiles: Vec<String> = app
            .state::<AppState>()
            .profiles
            .iter()
            .filter(|profile| profile.family == "claude")
            .map(|profile| profile.id.clone())
            .collect();
        for mut req in server.incoming_requests() {
            let url = req.url().to_string();
            let mut body = String::new();
            let _ = req
                .as_reader()
                .take(256 * 1024)
                .read_to_string(&mut body);
            if url.starts_with("/event") {
                let ev = parse(&url, &body, &claude_profiles);
                let state = app.state::<AppState>();
                let changed = {
                    let mut store = state.store.lock().unwrap();
                    store.apply(ev)
                };
                if changed {
                    crate::broadcast(&app);
                }
            }
            let _ = req.respond(tiny_http::Response::from_string("ok"));
        }
    });
}

fn query_param(url: &str, key: &str) -> String {
    let q = url.splitn(2, '?').nth(1).unwrap_or("");
    for pair in q.split('&') {
        let mut it = pair.splitn(2, '=');
        if it.next() == Some(key) {
            return it.next().unwrap_or("").to_string();
        }
    }
    String::new()
}

/// Only an id Notch discovered itself is accepted; anything else stays with the default profile
fn provider(requested: &str, claude_profiles: &[String]) -> String {
    claude_profiles
        .iter()
        .find(|id| *id == requested)
        .cloned()
        .unwrap_or_else(|| "claude".into())
}

fn parse(url: &str, body: &str, claude_profiles: &[String]) -> HookEvent {
    let v: serde_json::Value = serde_json::from_str(body).unwrap_or(serde_json::Value::Null);
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    // tool_input.command (Bash etc.) feeds the "last action" summary
    let tool_cmd = v
        .get("tool_input")
        .and_then(|t| t.get("command"))
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();
    HookEvent {
        e: query_param(url, "e"),
        session_id: {
            let id = s("session_id");
            if id.is_empty() { "unknown".into() } else { id }
        },
        provider: provider(&query_param(url, "profile"), claude_profiles),
        ppid: query_param(url, "ppid").parse().unwrap_or(0),
        cwd: s("cwd"),
        prompt: s("prompt"),
        message: s("message"),
        tool_name: s("tool_name"),
        tool_cmd,
        model: s("model"),
        src: "hook",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_events_belong_only_to_discovered_claude_profiles() {
        let known = vec!["claude-work".to_string()];
        for (url, expected) in [
            ("/event?e=done&ppid=7&profile=claude-work", "claude-work"),
            ("/event?e=done&ppid=7", "claude"),
            ("/event?e=done&profile=claude-unknown", "claude"),
            ("/event?e=done&profile=codex-work", "claude"),
        ] {
            let event = parse(url, r#"{"session_id":"abc"}"#, &known);
            assert_eq!(
                (event.session_id.as_str(), event.provider.as_str()),
                ("abc", expected),
                "{url}"
            );
        }
    }
}
