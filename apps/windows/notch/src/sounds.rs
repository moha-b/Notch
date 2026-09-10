use crate::AppState;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn play_session_sound(app: AppHandle, provider: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let settings = state.cfg.lock().unwrap();
    if !settings.sounds
        || settings.muted_providers.contains(&provider)
        || settings.disabled_providers.contains(&provider)
        || !settings.provider_order.contains(&provider)
    {
        return Ok(());
    }
    play_system_sound()
}

#[cfg(windows)]
fn play_system_sound() -> Result<(), String> {
    use windows::Win32::{
        System::Diagnostics::Debug::MessageBeep, UI::WindowsAndMessaging::MB_ICONINFORMATION,
    };
    // The system owns sound selection and respects the user's Windows sound settings.
    unsafe { MessageBeep(MB_ICONINFORMATION) }
        .map_err(|error| format!("Could not play session sound: {error}"))
}

#[cfg(not(windows))]
fn play_system_sound() -> Result<(), String> {
    Err("This sound adapter requires Windows.".into())
}
