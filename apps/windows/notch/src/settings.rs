use crate::{config::Config, providers, usage::UsageSnapshot, AppState};
use std::collections::{BTreeMap, HashSet};
use tauri::{AppHandle, Emitter, Manager};

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Config { app.state::<AppState>().cfg.lock().unwrap().clone() }

fn validate(settings: &Config) -> Result<(), String> {
    if !["left", "right", "top", "bottom"].contains(&settings.edge.as_str()) { return Err("Unknown screen edge.".into()); }
    if !settings.scale.is_finite() || !(0.5..=2.0).contains(&settings.scale) { return Err("Size must be between 50% and 200%.".into()); }
    if !settings.notch_y.is_finite() || !(0.0..=1.0).contains(&settings.notch_y) { return Err("Offset must be between 0 and 1.".into()); }
    if settings.peek_seconds > 60 { return Err("Peek duration must be at most 60 seconds.".into()); }
    if settings.accent.len() != 7 || !settings.accent.starts_with('#') || !settings.accent[1..].bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Accent must be a hex color.".into());
    }
    let order: HashSet<_> = settings.provider_order.iter().map(String::as_str).collect();
    if order.len() != providers::FAMILIES.len() || settings.provider_order.len() != order.len()
        || providers::FAMILIES.iter().any(|id| !order.contains(id)) { return Err("Provider order must contain each family exactly once.".into()); }
    if settings.disabled_providers.iter().chain(&settings.muted_providers).any(|id| !providers::FAMILIES.contains(&id.as_str())) {
        return Err("Unknown provider.".into());
    }
    Ok(())
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Config) -> Result<(), String> {
    validate(&settings)?;
    let state = app.state::<AppState>();
    let mut current = state.cfg.lock().unwrap();
    crate::config::try_save(&settings)?;
    *current = settings.clone();
    drop(current);
    crate::place_notch(&app);
    app.emit("settings", &settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_providers(app: AppHandle) -> BTreeMap<String, UsageSnapshot> { app.state::<AppState>().providers.lock().unwrap().clone() }

#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<(), String> {
    let window = if let Some(window) = app.get_webview_window("settings") { window } else {
        tauri::WebviewWindowBuilder::new(&app, "settings", tauri::WebviewUrl::App("settings.html".into()))
            .title(crate::identity::NAME).inner_size(740.0, 650.0).build().map_err(|e| e.to_string())?
    };
    window.show().and_then(|_| window.set_focus()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_displays(app: AppHandle) -> Result<Vec<String>, String> {
    app.available_monitors().map(|monitors| monitors.iter().filter_map(|monitor| monitor.name().cloned()).collect()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_ollama_key(key: String) -> Result<(), String> { providers::keyring::store(&key) }

#[tauri::command]
pub fn delete_ollama_key() -> Result<(), String> { providers::keyring::delete() }
