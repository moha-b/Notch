use super::{fresh_snapshot, home, json_file, Failure};
use crate::{codex, config::Config, usage::UsageSnapshot};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub family: &'static str,
    root: PathBuf,
}

impl Profile {
    /// The profile's own configuration directory, such as `~/.claude-work`.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

pub fn discover() -> Result<Vec<Profile>, Failure> {
    discover_in(&home()?)
}

fn discover_in(home: &Path) -> Result<Vec<Profile>, Failure> {
    let entries = std::fs::read_dir(home)
        .map_err(|_| Failure::Unavailable("Cannot discover provider profiles.".into()))?;
    let mut profiles = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|_| {
            Failure::Unavailable("Cannot inspect a provider profile directory.".into())
        })?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(profile) = profile_at(&name, entry.path()) {
            profiles.push(profile);
        }
    }
    profiles.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(profiles)
}

fn profile_at(directory: &str, root: PathBuf) -> Option<Profile> {
    // Only named profiles with provider-owned markers qualify; discovery never opens credentials.
    let (family, label, slug) = if let Some(slug) = directory.strip_prefix(".claude-") {
        ("claude", "Claude", slug)
    } else {
        ("codex", "Codex", directory.strip_prefix(".codex-")?)
    };
    if slug.is_empty() || !root.is_dir() || !has_profile_marker(family, &root) {
        return None;
    }
    Some(Profile {
        id: format!("{family}-{slug}"),
        name: format!("{label} ({slug})"),
        family,
        root,
    })
}

fn has_profile_marker(family: &str, root: &Path) -> bool {
    let markers: &[&str] = if family == "claude" {
        &[".credentials.json", "credentials.json"]
    } else {
        &[
            "auth.json",
            "config.toml",
            "sessions",
            "history.jsonl",
            "state_5.sqlite",
            "sqlite",
            "codex-dev.db",
        ]
    };
    markers.iter().any(|name| root.join(name).exists())
}

pub fn reconcile(settings: &mut Config, profiles: &[Profile]) {
    let known = |id: &String| {
        super::FAMILIES.contains(&id.as_str()) || profiles.iter().any(|profile| profile.id == *id)
    };
    settings.provider_order.retain(known);
    settings.disabled_providers.retain(known);
    settings.muted_providers.retain(known);
    for profile in profiles {
        if !settings.provider_order.contains(&profile.id) {
            settings.provider_order.push(profile.id.clone());
            if !settings.onboarding_complete && !settings.disabled_providers.contains(&profile.id) {
                settings.disabled_providers.push(profile.id.clone());
            }
        }
    }
}

pub fn read(profile: &Profile) -> Result<UsageSnapshot, Failure> {
    if profile.family == "claude" {
        return claude_reading(&profile.root);
    }
    match codex_reading(&profile.root) {
        Ok(snapshot) => Ok(snapshot),
        Err(failure @ Failure::RateLimited(_)) => Err(failure),
        Err(failure) => codex_history(&profile.root).ok_or(failure),
    }
}

fn claude_token(root: &Path) -> Result<String, Failure> {
    for filename in [".credentials.json", "credentials.json"] {
        let Some(credential) = json_file(&root.join(filename))? else {
            continue;
        };
        let oauth = credential.get("claudeAiOauth").unwrap_or(&credential);
        if let Some(token) = oauth
            .get("accessToken")
            .and_then(|token| token.as_str())
            .filter(|token| !token.trim().is_empty())
        {
            return Ok(token.to_owned());
        }
    }
    Err(Failure::NeedsAuth)
}

fn claude_reading(root: &Path) -> Result<UsageSnapshot, Failure> {
    let token = claude_token(root)?;
    let reply = crate::usage::profile_windows;
    let response = match reply(&token) {
        Err(Failure::NeedsAuth) => {
            let refreshed = claude_token(root)?;
            if refreshed == token {
                return Err(Failure::NeedsAuth);
            }
            reply(&refreshed)?
        }
        outcome => outcome?,
    };
    Ok(fresh_snapshot(response))
}

fn codex_reading(root: &Path) -> Result<UsageSnapshot, Failure> {
    let credential = json_file(&root.join("auth.json"))?.ok_or(Failure::NeedsAuth)?;
    codex::profile_reading(&credential)
}

fn codex_history(root: &Path) -> Option<UsageSnapshot> {
    let path = codex::newest_rollout_in(root)?;
    let (windows, recorded, plan) = codex::snapshot_from_rollout(&codex::tail_text(&path)?)?;
    Some(UsageSnapshot {
        status: "stale".into(),
        windows,
        fetched_at: recorded.unwrap_or(0),
        note: format!(
            "{} · Last recorded usage from this profile; live reading unavailable.",
            plan.unwrap_or_else(|| "Codex".into())
        ),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn profile_discovery_and_credentials_stay_isolated() {
        let directory = std::env::temp_dir().join(format!(
            "notch-profiles-{}-{}",
            std::process::id(),
            super::super::now_ms()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        for (slug, token) in [("work", "work-fixture"), ("personal", "personal-fixture")] {
            let profile = directory.join(format!(".claude-{slug}"));
            std::fs::create_dir(&profile).unwrap();
            std::fs::write(
                profile.join(".credentials.json"),
                json!({"claudeAiOauth":{"accessToken":token}}).to_string(),
            )
            .unwrap();
        }
        std::fs::create_dir(directory.join(".claude-plugins")).unwrap();
        std::fs::create_dir(directory.join(".codex-signed-out")).unwrap();
        std::fs::write(
            directory.join(".codex-signed-out/config.toml"),
            "model = 'example'",
        )
        .unwrap();
        let profiles = discover_in(&directory).unwrap();
        assert_eq!(
            profiles
                .iter()
                .map(|profile| profile.id.as_str())
                .collect::<Vec<_>>(),
            vec!["claude-personal", "claude-work", "codex-signed-out"]
        );
        assert_eq!(claude_token(&profiles[0].root).unwrap(), "personal-fixture");
        assert_eq!(claude_token(&profiles[1].root).unwrap(), "work-fixture");
        std::fs::remove_file(profiles[0].root.join(".credentials.json")).unwrap();
        assert!(matches!(
            claude_token(&profiles[0].root),
            Err(Failure::NeedsAuth)
        ));
        assert_eq!(claude_token(&profiles[1].root).unwrap(), "work-fixture");
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn profiles_created_after_launch_wait_for_the_next_discovery() {
        let directory = std::env::temp_dir().join(format!(
            "notch-late-profile-{}-{}",
            std::process::id(),
            super::super::now_ms()
        ));
        let create = |slug: &str| {
            let profile = directory.join(format!(".claude-{slug}"));
            std::fs::create_dir_all(&profile).unwrap();
            std::fs::write(profile.join(".credentials.json"), "{}").unwrap();
        };
        let ids = |profiles: &[Profile]| profiles.iter().map(|p| p.id.clone()).collect::<Vec<_>>();
        create("work");
        let at_launch = discover_in(&directory).unwrap();
        create("late");
        assert_eq!(ids(&at_launch), vec!["claude-work"]);
        let relaunched = discover_in(&directory).unwrap();
        assert_eq!(ids(&relaunched), vec!["claude-late", "claude-work"]);

        let mut onboarded = Config {
            onboarding_complete: true,
            ..Config::default()
        };
        reconcile(&mut onboarded, &relaunched);
        assert!(onboarded
            .provider_order
            .contains(&"claude-late".to_string()));
        assert!(!onboarded
            .disabled_providers
            .contains(&"claude-late".to_string()));
        let mut first_run = Config::default();
        reconcile(&mut first_run, &relaunched);
        assert!(first_run
            .disabled_providers
            .contains(&"claude-late".to_string()));
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn codex_fallback_keeps_each_profiles_recorded_usage_and_age() {
        let directory = std::env::temp_dir().join(format!(
            "notch-history-{}-{}",
            std::process::id(),
            super::super::now_ms()
        ));
        for (slug, percent) in [("work", 23), ("personal", 81)] {
            let root = directory.join(format!(".codex-{slug}"));
            let logs = root.join("sessions/2026/09/10");
            std::fs::create_dir_all(&logs).unwrap();
            let event = json!({"timestamp":"2026-09-10T00:00:00Z","payload":{"rate_limits":{"primary":{"used_percent":percent,"window_minutes":300,"resets_at":1800000000}}}});
            let log = logs.join("rollout-fixture.jsonl");
            let original = format!("{event}\n{{\"partial\":");
            std::fs::write(&log, &original).unwrap();
            let profile = profile_at(&format!(".codex-{slug}"), root).unwrap();
            let reading = read(&profile).unwrap();
            assert_eq!(reading.status, "stale");
            assert_eq!(reading.windows[0].used, f64::from(percent) / 100.0);
            assert_eq!(reading.fetched_at, 1788998400000);
            assert_eq!(std::fs::read_to_string(log).unwrap(), original);
        }
        std::fs::remove_dir_all(&directory).unwrap();
    }
}
