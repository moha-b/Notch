//! Merges notch-hook.exe into ~/.claude/settings.json without overwriting the user's own hooks.
//! Ownership requires the exact helper path and a registered event. A backup is written first.

use serde_json::{json, Value};
use std::path::PathBuf;

/// (Claude Code event name, whether it needs a matcher, the internal event reported to Notch)
const WIRING: &[(&str, bool, &str)] = &[
    ("SessionStart", false, "session_start"),
    ("UserPromptSubmit", false, "running"),
    ("PreToolUse", true, "running"),
    ("PostToolUse", true, "running"),
    ("Notification", false, "attention"),
    ("Stop", false, "done"),
    ("SessionEnd", false, "session_end"),
];

fn settings_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("settings.json"))
}

fn load(path: &PathBuf) -> Result<Value, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(e.to_string()),
    }
}

fn backup_and_write(path: &PathBuf, root: &Value) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    if path.exists() {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        std::fs::copy(path, path.with_extension(format!("json.notch-bak-{ts}")))
            .map_err(|e| e.to_string())?;
    }
    let txt = serde_json::to_string_pretty(root).map_err(|e| e.to_string())?;
    std::fs::write(path, txt).map_err(|e| e.to_string())
}

pub fn is_installed() -> bool {
    let Ok(exe) = std::env::current_exe() else { return false };
    let Some(path) = settings_path() else { return false };
    let Ok(mut root) = load(&path) else { return false };
    remove_installed_hooks(&mut root, &exe.with_file_name("notch-hook.exe")) > 0
}

pub fn install() -> Result<String, String> {
    let path = settings_path().ok_or("cannot find the user directory")?;
    let hook_exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("cannot locate the program directory")?
        .join("notch-hook.exe");
    if !hook_exe.exists() {
        return Err(format!("missing {}", hook_exe.display()));
    }

    let mut root = load(&path)?;
    merge_hooks(&mut root, &hook_exe)?;
    backup_and_write(&path, &root)?;
    Ok(format!("wrote {} ({} events)", path.display(), WIRING.len()))
}

fn validate_hooks(root: &Value) -> Result<(), String> {
    if !root.is_object() { return Err("settings.json must be an object".into()); }
    if !root["hooks"].is_null() && !root["hooks"].is_object() {
        return Err("hooks must be an object".into());
    }
    for (event, _, _) in WIRING {
        if !root["hooks"][*event].is_null() && !root["hooks"][*event].is_array() {
            return Err(format!("hooks.{event} must be an array"));
        }
    }
    Ok(())
}

fn merge_hooks(root: &mut Value, hook_exe: &std::path::Path) -> Result<(), String> {
    validate_hooks(root)?;
    if root["hooks"].is_null() { root["hooks"] = json!({}); }
    remove_installed_hooks(root, hook_exe);
    for (event, need_matcher, internal) in WIRING {
        let mut groups = root["hooks"][*event].as_array().cloned().unwrap_or_default();
        let command = format!("\"{}\" {}", hook_exe.display(), internal);
        let mut group = json!({"hooks": [{"type": "command", "command": command, "timeout": 5}]});
        if *need_matcher { group["matcher"] = json!("*"); }
        groups.push(group);
        root["hooks"][*event] = json!(groups);
    }
    Ok(())
}

pub fn uninstall() -> Result<String, String> {
    uninstall_current_installation()
}

/// The installer must not remove hooks owned by another checkout or mixed into the same group.
pub fn uninstall_current_installation() -> Result<String, String> {
    let path = settings_path().ok_or("cannot find the user directory")?;
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let helper = executable.with_file_name("notch-hook.exe");
    uninstall_from(&path, &helper)
}

fn uninstall_from(path: &PathBuf, helper: &std::path::Path) -> Result<String, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok("no settings to clean".into()),
        Err(e) => return Err(e.to_string()),
    };
    let mut root: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let removed = remove_installed_hooks(&mut root, helper);
    if removed > 0 {
        backup_and_write(path, &root)?;
    }
    Ok(format!("removed {removed} installed Notch hook(s)"))
}

fn remove_installed_hooks(root: &mut Value, helper: &std::path::Path) -> usize {
    let prefix = format!("\"{}\" ", helper.display());
    let Some(events) = root.get_mut("hooks").and_then(Value::as_object_mut) else { return 0 };
    let mut removed = 0;
    for groups in events.values_mut().filter_map(Value::as_array_mut) {
        groups.retain_mut(|group| {
            let Some(commands) = group.get_mut("hooks").and_then(Value::as_array_mut) else { return true };
            let before = commands.len();
            commands.retain(|hook| {
                !hook["command"].as_str().map(|command| {
                    command.strip_prefix(&prefix)
                        .map(|event| WIRING.iter().any(|(_, _, internal)| event == *internal))
                        .unwrap_or(false)
                }).unwrap_or(false)
            });
            removed += before - commands.len();
            before == commands.len() || !commands.is_empty()
        });
    }
    removed
}

#[cfg(test)]
mod installer_tests {
    use super::*;

    #[test]
    fn install_is_idempotent_and_preserves_upstream_and_mixed_hooks() {
        let helper = std::path::Path::new(r"C:\Notch\notch-hook.exe");
        let upstream = json!({"type": "command", "command": "\"C:\\Codenotch\\codenotch-hook.exe\" done"});
        let unrelated = json!({"command": "echo keep"});
        let mut root = json!({"hooks": {"Stop": [{"hooks": [
            upstream.clone(), unrelated.clone(), {"command": "\"C:\\Notch\\notch-hook.exe\" done"}
        ]}]}, "unrelated": true});
        merge_hooks(&mut root, helper).unwrap();
        let once = root.clone();
        merge_hooks(&mut root, helper).unwrap();
        assert_eq!(root, once);
        assert_eq!(root["hooks"]["Stop"][0]["hooks"], json!([upstream, unrelated]));
        assert_eq!(root["unrelated"], true);
    }

    #[test]
    fn invalid_hook_shapes_are_rejected_without_mutation() {
        for mut root in [json!([]), json!({"hooks": []}), json!({"hooks": {"Stop": "invalid"}})] {
            let before = root.clone();
            assert!(merge_hooks(&mut root, std::path::Path::new("notch-hook.exe")).is_err());
            assert_eq!(root, before);
        }
    }

    #[test]
    fn uninstall_preserves_other_commands_and_installations() {
        let helper = std::path::Path::new(r"C:\Users\Zoë Test\Notch\notch-hook.exe");
        let mut root = json!({"other": true, "hooks": {"Stop": [
            {"matcher": "*", "hooks": [
                {"command": format!("\"{}\" done", helper.display())},
                {"command": "echo keep me"},
                {"command": "\"C:\\Other\\notch-hook.exe\" done"}
            ]},
            {"hooks": [{"command": format!("\"{}\" done", helper.display())}]},
            {"hooks": []}
        ]}});
        assert_eq!(remove_installed_hooks(&mut root, helper), 2);
        assert_eq!(root, json!({"other": true, "hooks": {"Stop": [
            {"matcher": "*", "hooks": [
                {"command": "echo keep me"},
                {"command": "\"C:\\Other\\notch-hook.exe\" done"}
            ]}, {"hooks": []}
        ]}}));
        assert_eq!(remove_installed_hooks(&mut root, helper), 0);
    }
}
