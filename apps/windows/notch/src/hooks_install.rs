//! Merges notch-hook.exe into Claude Code settings without overwriting the user's own hooks: the default
//! ~/.claude/settings.json plus each named profile's own settings.json, whose commands carry the profile id
//! so its sessions land on its own ring. Ownership requires the exact helper path and a registered event.
//! A backup is written first.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

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

/// One settings file to wire, and the profile id its hook commands report (None for the default profile)
struct Target {
    path: PathBuf,
    profile: Option<String>,
}

fn default_target() -> Result<Target, String> {
    let home = dirs::home_dir().ok_or("cannot find the user directory")?;
    Ok(Target { path: home.join(".claude").join("settings.json"), profile: None })
}

/// The default settings plus every named Claude profile discovered now (a later one is wired on the next
/// install), and the ids of profiles whose names cannot travel safely on a hook command line
fn targets() -> Result<(Vec<Target>, Vec<String>), String> {
    let profiles = crate::providers::profiles::discover()
        .map_err(|_| "cannot discover Claude profiles".to_string())?;
    let mut targets = vec![default_target()?];
    let mut unsupported = Vec::new();
    for profile in profiles.iter().filter(|profile| profile.family == "claude") {
        if crate::identity::is_hook_profile_id(&profile.id) {
            targets.push(Target {
                path: profile.root().join("settings.json"),
                profile: Some(profile.id.clone()),
            });
        } else {
            unsupported.push(profile.id.clone());
        }
    }
    Ok((targets, unsupported))
}

fn helper_path() -> Result<PathBuf, String> {
    Ok(std::env::current_exe().map_err(|e| e.to_string())?.with_file_name("notch-hook.exe"))
}

fn load(path: &Path) -> Result<Value, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(e.to_string()),
    }
}

fn backup_and_write(path: &Path, root: &Value) -> Result<(), String> {
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
    let (Ok(helper), Ok(target)) = (helper_path(), default_target()) else { return false };
    load(&target.path)
        .map(|mut root| remove_installed_hooks(&mut root, &helper) > 0)
        .unwrap_or(false)
}

pub fn install() -> Result<String, String> {
    let hook_exe = helper_path()?;
    if !hook_exe.exists() {
        return Err(format!("missing {}", hook_exe.display()));
    }
    let (targets, unsupported) = targets()?;
    let written = install_into(&targets, &hook_exe)?;
    let mut message = format!("wrote {written} settings file(s) ({} events each)", WIRING.len());
    if !unsupported.is_empty() {
        message += &format!("; profiles with unsupported names were not wired: {}", unsupported.join(", "));
    }
    Ok(message)
}

/// Every file is merged before any is written, so one malformed profile leaves all of them untouched
fn install_into(targets: &[Target], hook_exe: &Path) -> Result<usize, String> {
    let mut merged = Vec::with_capacity(targets.len());
    for target in targets {
        let context = |error: String| format!("{}: {error}", target.path.display());
        let mut root = load(&target.path).map_err(context)?;
        merge_hooks(&mut root, hook_exe, target.profile.as_deref()).map_err(context)?;
        merged.push((&target.path, root));
    }
    for (path, root) in &merged {
        backup_and_write(path, root)?;
    }
    Ok(merged.len())
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

fn merge_hooks(root: &mut Value, hook_exe: &Path, profile: Option<&str>) -> Result<(), String> {
    validate_hooks(root)?;
    if root["hooks"].is_null() { root["hooks"] = json!({}); }
    remove_installed_hooks(root, hook_exe);
    for (event, need_matcher, internal) in WIRING {
        let mut groups = root["hooks"][*event].as_array().cloned().unwrap_or_default();
        let command = match profile {
            Some(id) => format!("\"{}\" {internal} {id}", hook_exe.display()),
            None => format!("\"{}\" {internal}", hook_exe.display()),
        };
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
/// A failed profile discovery still cleans the default settings.
pub fn uninstall_current_installation() -> Result<String, String> {
    let helper = helper_path()?;
    let targets = match targets() {
        Ok((targets, _)) => targets,
        Err(_) => vec![default_target()?],
    };
    let mut removed = 0;
    for target in &targets {
        removed += uninstall_from(&target.path, &helper)?;
    }
    Ok(format!("removed {removed} installed Notch hook(s) from {} settings file(s)", targets.len()))
}

fn uninstall_from(path: &Path, helper: &Path) -> Result<usize, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e.to_string()),
    };
    let mut root: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let removed = remove_installed_hooks(&mut root, helper);
    if removed > 0 {
        backup_and_write(path, &root)?;
    }
    Ok(removed)
}

/// `"<helper>" <event>` or `"<helper>" <event> <profile id>` from this exact helper; nothing else is ours
fn is_owned_command(command: &str, prefix: &str) -> bool {
    let Some(rest) = command.strip_prefix(prefix) else { return false };
    let mut words = rest.split(' ');
    let event = words.next().unwrap_or("");
    let profile = words.next();
    WIRING.iter().any(|(_, _, internal)| event == *internal)
        && profile.is_none_or(crate::identity::is_hook_profile_id)
        && words.next().is_none()
}

fn remove_installed_hooks(root: &mut Value, helper: &Path) -> usize {
    let prefix = format!("\"{}\" ", helper.display());
    let Some(events) = root.get_mut("hooks").and_then(Value::as_object_mut) else { return 0 };
    let mut removed = 0;
    for groups in events.values_mut().filter_map(Value::as_array_mut) {
        groups.retain_mut(|group| {
            let Some(commands) = group.get_mut("hooks").and_then(Value::as_array_mut) else { return true };
            let before = commands.len();
            commands.retain(|hook| {
                !hook["command"].as_str().is_some_and(|command| is_owned_command(command, &prefix))
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
        let helper = Path::new(r"C:\Notch\notch-hook.exe");
        let upstream = json!({"type": "command", "command": "\"C:\\Codenotch\\codenotch-hook.exe\" done"});
        let unrelated = json!({"command": "echo keep"});
        let mut root = json!({"hooks": {"Stop": [{"hooks": [
            upstream.clone(), unrelated.clone(), {"command": "\"C:\\Notch\\notch-hook.exe\" done"}
        ]}]}, "unrelated": true});
        merge_hooks(&mut root, helper, None).unwrap();
        let once = root.clone();
        merge_hooks(&mut root, helper, None).unwrap();
        assert_eq!(root, once);
        assert_eq!(root["hooks"]["Stop"][0]["hooks"], json!([upstream, unrelated]));
        assert_eq!(root["unrelated"], true);
    }

    #[test]
    fn invalid_hook_shapes_are_rejected_without_mutation() {
        for mut root in [json!([]), json!({"hooks": []}), json!({"hooks": {"Stop": "invalid"}})] {
            let before = root.clone();
            assert!(merge_hooks(&mut root, Path::new("notch-hook.exe"), None).is_err());
            assert_eq!(root, before);
        }
    }

    #[test]
    fn uninstall_preserves_other_commands_and_installations() {
        let helper = Path::new(r"C:\Users\Zoë Test\Notch\notch-hook.exe");
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

    #[test]
    fn profile_hooks_carry_their_id_and_leave_other_helpers_alone() {
        let helper = Path::new(r"C:\Notch\notch-hook.exe");
        let other = json!({"command": "\"C:\\Other\\notch-hook.exe\" done claude-work"});
        let mut root = json!({"hooks": {"Stop": [{"hooks": [other.clone()]}]}});
        merge_hooks(&mut root, helper, Some("claude-work")).unwrap();
        let once = root.clone();
        merge_hooks(&mut root, helper, Some("claude-work")).unwrap();
        assert_eq!(root, once);
        assert_eq!(
            root["hooks"]["Stop"][1]["hooks"][0]["command"],
            r#""C:\Notch\notch-hook.exe" done claude-work"#
        );
        assert_eq!(remove_installed_hooks(&mut root, helper), WIRING.len());
        assert_eq!(root["hooks"]["Stop"], json!([{"hooks": [other]}]));
    }

    #[test]
    fn only_command_line_safe_profile_ids_are_wired() {
        let too_long = format!("claude-{}", "x".repeat(60));
        for id in ["claude-work", "claude-Client.2_b-c"] {
            assert!(crate::identity::is_hook_profile_id(id), "{id}");
        }
        for id in ["claude", "claude-", "codex-work", "claude-my work", "claude-a&b", too_long.as_str()] {
            assert!(!crate::identity::is_hook_profile_id(id), "{id}");
        }
        let prefix = r#""C:\Notch\notch-hook.exe" "#;
        assert!(!is_owned_command(&format!("{prefix}done claude-my work"), prefix));
        assert!(!is_owned_command(&format!("{prefix}unknown claude-work"), prefix));
    }

    #[test]
    fn every_settings_file_is_validated_before_any_is_written() {
        let directory = std::env::temp_dir().join(format!(
            "notch-hooks-{}-{}",
            std::process::id(),
            crate::providers::now_ms()
        ));
        let default = directory.join(".claude").join("settings.json");
        let work = directory.join(".claude-work").join("settings.json");
        for path in [&default, &work] {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        }
        std::fs::write(&default, r#"{"keep":true}"#).unwrap();
        std::fs::write(&work, r#"{"hooks":[]}"#).unwrap();
        let targets = [
            Target { path: default.clone(), profile: None },
            Target { path: work.clone(), profile: Some("claude-work".into()) },
        ];
        let helper = Path::new(r"C:\Notch\notch-hook.exe");
        assert!(install_into(&targets, helper).is_err());
        assert_eq!(std::fs::read_to_string(&default).unwrap(), r#"{"keep":true}"#);

        std::fs::write(&work, "{}").unwrap();
        assert_eq!(install_into(&targets, helper).unwrap(), 2);
        let read = |path: &Path| -> Value { serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap() };
        assert_eq!(read(&default)["keep"], true);
        assert_eq!(read(&default)["hooks"]["Stop"][0]["hooks"][0]["command"], r#""C:\Notch\notch-hook.exe" done"#);
        assert_eq!(
            read(&work)["hooks"]["Stop"][0]["hooks"][0]["command"],
            r#""C:\Notch\notch-hook.exe" done claude-work"#
        );
        let _ = std::fs::remove_dir_all(&directory);
    }
}
