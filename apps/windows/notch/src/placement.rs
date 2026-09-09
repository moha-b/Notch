use crate::{config::Config, AppState};
use tauri::{AppHandle, Manager, Monitor};

fn selected_monitor(app: &AppHandle, settings: &Config) -> Option<Monitor> {
    if settings.follow_focus {
        if let Some((x, y)) = foreground_center() {
            if let Ok(Some(monitor)) = app.monitor_from_point(x, y) { return Some(monitor); }
        }
    }
    if let Some(name) = &settings.display {
        if let Some(monitor) = app.available_monitors().ok()?.into_iter().find(|monitor| monitor.name() == Some(name)) {
            return Some(monitor);
        }
    }
    app.primary_monitor().ok().flatten()
}

#[cfg(windows)]
fn foreground_center() -> Option<(f64, f64)> {
    use windows::Win32::{Foundation::RECT, UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect}};
    let mut rectangle = RECT::default();
    unsafe { GetWindowRect(GetForegroundWindow(), &mut rectangle).ok()?; }
    Some(((rectangle.left as f64 + rectangle.right as f64) / 2.0, (rectangle.top as f64 + rectangle.bottom as f64) / 2.0))
}

#[cfg(not(windows))]
fn foreground_center() -> Option<(f64, f64)> { None }

pub fn place(app: &AppHandle) {
    let Some(window) = app.get_webview_window("notch") else { return };
    let settings = app.state::<AppState>().cfg.lock().unwrap().clone();
    let Some(monitor) = selected_monitor(app, &settings) else { return };
    let horizontal = matches!(settings.edge.as_str(), "top" | "bottom");
    let physical = monitor.size();
    let scale = monitor.scale_factor() * settings.scale;
    let width = if horizontal { physical.width } else { (340.0 * scale).round() as u32 }.min(physical.width);
    let height = if horizontal { (460.0 * scale).round() as u32 } else { physical.height }.min(physical.height);
    let origin = monitor.position();
    let offset_x = ((physical.width - width) as f64 * settings.notch_y).round() as i32;
    let offset_y = ((physical.height - height) as f64 * settings.notch_y).round() as i32;
    let (x, y) = match settings.edge.as_str() {
        "left" => (origin.x, origin.y + offset_y),
        "top" => (origin.x + offset_x, origin.y),
        "bottom" => (origin.x + offset_x, origin.y + physical.height as i32 - height as i32),
        _ => (origin.x + physical.width as i32 - width as i32, origin.y + offset_y),
    };
    let size = tauri::PhysicalSize::new(width, height);
    let position = tauri::PhysicalPosition::new(x, y);
    if window.outer_size().ok() != Some(size) { let _ = window.set_size(size); }
    if window.outer_position().ok() != Some(position) { let _ = window.set_position(position); }
}

pub fn watch(app: AppHandle) {
    std::thread::spawn(move || loop {
        place(&app);
        std::thread::sleep(std::time::Duration::from_secs(2));
    });
}
