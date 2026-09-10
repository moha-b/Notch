use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

#[derive(Clone, Default, Serialize)]
pub struct Status {
    phase: String,
    version: Option<String>,
    notes: Option<String>,
    error: Option<String>,
}

#[derive(Default)]
struct Pending {
    status: Status,
    update: Option<Update>,
    bytes: Option<Vec<u8>>,
}

#[derive(Default)]
pub struct Updates(tauri::async_runtime::Mutex<Pending>);

fn require_stable() -> Result<(), String> {
    if crate::identity::STABLE {
        Ok(())
    } else {
        Err("Updates are disabled in development builds.".into())
    }
}

fn announce(app: &AppHandle, status: &Status) {
    let _ = app.emit("update-status", status);
}

#[tauri::command]
pub async fn update_status(app: AppHandle) -> Status {
    if !crate::identity::STABLE {
        return Status {
            phase: "development".into(),
            ..Default::default()
        };
    }
    let state = app.state::<Updates>();
    let status = state.0.lock().await.status.clone();
    status
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Status, String> {
    require_stable()?;
    let state = app.state::<Updates>();
    let mut pending = state.0.lock().await;
    if pending.bytes.is_some() {
        return Ok(pending.status.clone());
    }
    let checked = app
        .updater()
        .map_err(|error| error.to_string())?
        .check()
        .await;
    match checked {
        Ok(update) => {
            pending.status = Status {
                phase: if update.is_some() {
                    "available"
                } else {
                    "current"
                }
                .into(),
                version: update.as_ref().map(|update| update.version.clone()),
                notes: update.as_ref().and_then(|update| update.body.clone()),
                error: None,
            };
            pending.update = update;
        }
        Err(error) => pending.status.error = Some(format!("Cannot check for updates: {error}")),
    }
    announce(&app, &pending.status);
    Ok(pending.status.clone())
}

#[tauri::command]
pub async fn download_update(app: AppHandle) -> Result<(), String> {
    require_stable()?;
    let state = app.state::<Updates>();
    let mut pending = state.0.lock().await;
    let update = pending.update.clone().ok_or("Check for an update first.")?;
    pending.status.phase = "downloading".into();
    announce(&app, &pending.status);
    match update.download(|_, _| {}, || {}).await {
        Ok(bytes) => {
            pending.bytes = Some(bytes);
            pending.status.phase = "ready".into();
            pending.status.error = None;
        }
        Err(error) => {
            pending.status.phase = "available".into();
            pending.status.error = Some(error.to_string());
        }
    }
    announce(&app, &pending.status);
    Ok(())
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    require_stable()?;
    let state = app.state::<Updates>();
    let pending = state.0.lock().await;
    let update = pending.update.as_ref().ok_or("No update is available.")?;
    let bytes = pending
        .bytes
        .as_ref()
        .ok_or("Download and verify the update first.")?;
    update.install(bytes).map_err(|error| error.to_string())
}

pub fn start(app: AppHandle) {
    if !crate::identity::STABLE {
        return;
    }
    std::thread::spawn(move || {
        let mut next_check = 0;
        loop {
            let enabled = app
                .state::<crate::AppState>()
                .cfg
                .lock()
                .unwrap()
                .automatic_checks;
            if enabled && crate::providers::now_ms() >= next_check {
                let _ = tauri::async_runtime::block_on(check_update(app.clone()));
                next_check = crate::providers::now_ms() + 86_400_000;
            }
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    });
}
