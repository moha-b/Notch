use crate::{config::Config, providers, usage::UsageSnapshot, AppState};
use std::collections::{BTreeMap, HashSet};
use tauri::{AppHandle, Emitter, Manager};

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Config {
    app.state::<AppState>().cfg.lock().unwrap().clone()
}

fn validate(settings: &Config, profiles: &[providers::profiles::Profile]) -> Result<(), String> {
    settings.validate()?;
    let order: HashSet<_> = settings.provider_order.iter().map(String::as_str).collect();
    let known: HashSet<_> = providers::FAMILIES
        .iter()
        .copied()
        .chain(profiles.iter().map(|profile| profile.id.as_str()))
        .collect();
    if order != known || settings.provider_order.len() != order.len() {
        return Err("Provider order must contain each provider and profile exactly once.".into());
    }
    if settings
        .disabled_providers
        .iter()
        .chain(&settings.muted_providers)
        .any(|id| !known.contains(id.as_str()))
    {
        return Err("Unknown provider.".into());
    }
    Ok(())
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Config) -> Result<(), String> {
    let state = app.state::<AppState>();
    validate(&settings, &state.profiles)?;
    let mut current = state.cfg.lock().unwrap();
    crate::config::try_save(&settings)?;
    *current = settings.clone();
    drop(current);
    // Placement may open or close per-display windows, which must not happen on this command's thread
    let placer = app.clone();
    std::thread::spawn(move || crate::place_notch(&placer));
    app.emit("settings", &settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_profile_names(app: AppHandle) -> BTreeMap<String, String> {
    app.state::<AppState>()
        .profiles
        .iter()
        .map(|profile| (profile.id.clone(), profile.name.clone()))
        .collect()
}

#[tauri::command]
pub fn get_providers(app: AppHandle) -> BTreeMap<String, UsageSnapshot> {
    app.state::<AppState>().providers.lock().unwrap().clone()
}

#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<(), String> {
    let window = if let Some(window) = app.get_webview_window("settings") {
        window
    } else {
        tauri::WebviewWindowBuilder::new(
            &app,
            "settings",
            tauri::WebviewUrl::App("settings.html".into()),
        )
        .title(crate::identity::NAME)
        .inner_size(740.0, 650.0)
        .build()
        .map_err(|e| e.to_string())?
    };
    window
        .show()
        .and_then(|_| window.set_focus())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_displays(app: AppHandle) -> Result<Vec<String>, String> {
    app.available_monitors()
        .map(|monitors| {
            monitors
                .iter()
                .filter_map(|monitor| monitor.name().cloned())
                .collect()
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_ollama_key(key: String) -> Result<(), String> {
    providers::keyring::store(&key)
}

#[tauri::command]
pub fn delete_ollama_key() -> Result<(), String> {
    providers::keyring::delete()
}
