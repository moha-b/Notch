use crate::{config::Config, AppState};
use tauri::{AppHandle, Manager, Monitor, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// The window from tauri.conf.json: the only notch on Main display, the primary one's under All displays.
pub const MAIN_WINDOW: &str = "notch";

/// A monitor or window rectangle in physical pixels, kept apart from Tauri's types so layout is testable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

fn monitor_frame(monitor: &Monitor) -> Frame {
    Frame {
        x: monitor.position().x,
        y: monitor.position().y,
        width: monitor.size().width,
        height: monitor.size().height,
    }
}

fn selected_monitor(app: &AppHandle, settings: &Config) -> Option<Monitor> {
    if settings.follow_focus {
        if let Some((x, y)) = foreground_center() {
            if let Ok(Some(monitor)) = app.monitor_from_point(x, y) {
                return Some(monitor);
            }
        }
    }
    if let Some(name) = &settings.display {
        if let Some(monitor) = app
            .available_monitors()
            .ok()?
            .into_iter()
            .find(|monitor| monitor.name() == Some(name))
        {
            return Some(monitor);
        }
    }
    app.primary_monitor().ok().flatten()
}

/// Under All displays the main window stays on the primary monitor; choosing a monitor or following
/// the active window only applies to Main display, as on Mac.
fn preferred_monitor(app: &AppHandle, settings: &Config) -> Option<Monitor> {
    if settings.scope == "all_displays" {
        app.primary_monitor().ok().flatten()
    } else {
        selected_monitor(app, settings)
    }
}

#[cfg(windows)]
fn foreground_center() -> Option<(f64, f64)> {
    use windows::Win32::{
        Foundation::RECT,
        UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect},
    };
    let mut rectangle = RECT::default();
    unsafe {
        GetWindowRect(GetForegroundWindow(), &mut rectangle).ok()?;
    }
    Some((
        (rectangle.left as f64 + rectangle.right as f64) / 2.0,
        (rectangle.top as f64 + rectangle.bottom as f64) / 2.0,
    ))
}

#[cfg(not(windows))]
fn foreground_center() -> Option<(f64, f64)> {
    None
}

/// Which window sits on which monitor index. The preferred monitor always keeps the main window;
/// under All displays each other monitor gets `notch-<n>` from its position in the monitor list.
pub fn desired_windows(
    all_displays: bool,
    monitor_count: usize,
    preferred: usize,
) -> Vec<(String, usize)> {
    let mut windows = vec![(MAIN_WINDOW.to_string(), preferred)];
    if all_displays {
        windows.extend(
            (0..monitor_count)
                .filter(|index| *index != preferred)
                .map(|index| (format!("{MAIN_WINDOW}-{}", index + 1), index)),
        );
    }
    windows
}

/// The notch window on one monitor: welded to the edge along its full length, 340 or 460 logical
/// pixels deep at that monitor's own scale. The page places the pill along the edge from the offset.
pub fn window_frame(
    edge: &str,
    size_scale: f64,
    along: f64,
    monitor: Frame,
    monitor_scale: f64,
) -> Frame {
    let horizontal = matches!(edge, "top" | "bottom");
    let scale = monitor_scale * size_scale;
    let width = if horizontal {
        monitor.width
    } else {
        (340.0 * scale).round() as u32
    }
    .min(monitor.width);
    let height = if horizontal {
        (460.0 * scale).round() as u32
    } else {
        monitor.height
    }
    .min(monitor.height);
    let offset_x = ((monitor.width - width) as f64 * along).round() as i32;
    let offset_y = ((monitor.height - height) as f64 * along).round() as i32;
    let (x, y) = match edge {
        "left" => (monitor.x, monitor.y + offset_y),
        "top" => (monitor.x + offset_x, monitor.y),
        "bottom" => (
            monitor.x + offset_x,
            monitor.y + monitor.height as i32 - height as i32,
        ),
        _ => (
            monitor.x + monitor.width as i32 - width as i32,
            monitor.y + offset_y,
        ),
    };
    Frame {
        x,
        y,
        width,
        height,
    }
}

/// Where the pointer sits along the notch's edge of its own monitor, as the fleet-wide 0–1 offset.
pub fn along_ratio(edge: &str, monitor: Frame, cursor: (f64, f64)) -> f64 {
    let (position, start, length) = if matches!(edge, "top" | "bottom") {
        (cursor.0, monitor.x, monitor.width)
    } else {
        (cursor.1, monitor.y, monitor.height)
    };
    if length == 0 {
        return 0.5;
    }
    ((position - start as f64) / length as f64).clamp(0.0, 1.0)
}

/// The frame of the monitor a window is on now, for dragging along that monitor's edge.
pub fn current_frame(window: &WebviewWindow) -> Option<Frame> {
    window
        .current_monitor()
        .ok()
        .flatten()
        .map(|monitor| monitor_frame(&monitor))
}

/// Opens, moves, shows or hides every notch window so the fleet matches the settings and the
/// monitors connected right now. Runs off the command thread: opening a window inside a synchronous
/// command can deadlock on Windows.
pub fn place(app: &AppHandle) {
    let settings = app.state::<AppState>().cfg.lock().unwrap().clone();
    let Ok(monitors) = app.available_monitors() else {
        return;
    };
    if monitors.is_empty() {
        return;
    }
    let preferred = preferred_monitor(app, &settings)
        .and_then(|chosen| {
            monitors
                .iter()
                .position(|monitor| monitor.position() == chosen.position())
        })
        .unwrap_or(0);
    let wanted = desired_windows(settings.scope == "all_displays", monitors.len(), preferred);
    retire_unwanted(app, &wanted);
    for (label, index) in &wanted {
        let Some(window) = notch_window(app, label) else {
            continue;
        };
        let monitor = &monitors[*index];
        let frame = window_frame(
            &settings.edge,
            settings.scale,
            settings.notch_y,
            monitor_frame(monitor),
            monitor.scale_factor(),
        );
        apply_frame(&window, frame);
        set_shown(&window, settings.visibility != "hidden");
    }
}

/// The existing window for a label, or a new one built like the configured main window.
fn notch_window(app: &AppHandle, label: &str) -> Option<WebviewWindow> {
    if let Some(window) = app.get_webview_window(label) {
        return Some(window);
    }
    let window = WebviewWindowBuilder::new(app, label, WebviewUrl::App("notch.html".into()))
        .inner_size(340.0, 460.0)
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .visible(false)
        .focused(false)
        .build()
        .map_err(|error| crate::applog(&format!("could not open notch window {label}: {error}")))
        .ok()?;
    crate::noactivate(&window);
    if let Err(error) = crate::hit_regions::clear(&window) {
        crate::applog(&format!("{label}: {error}"));
    }
    Some(window)
}

/// A display's extra window closes when its monitor or All displays goes away; the main one never does.
fn retire_unwanted(app: &AppHandle, wanted: &[(String, usize)]) {
    for (label, window) in app.webview_windows() {
        let extra = label.starts_with(&format!("{MAIN_WINDOW}-"));
        if extra && !wanted.iter().any(|(kept, _)| *kept == label) {
            let _ = window.close();
        }
    }
}

fn apply_frame(window: &WebviewWindow, frame: Frame) {
    let size = tauri::PhysicalSize::new(frame.width, frame.height);
    let position = tauri::PhysicalPosition::new(frame.x, frame.y);
    if window.outer_size().ok() != Some(size) {
        let _ = window.set_size(size);
    }
    if window.outer_position().ok() != Some(position) {
        let _ = window.set_position(position);
    }
}

/// Hide takes the windows off screen entirely, so an invisible notch cannot keep catching the pointer.
fn set_shown(window: &WebviewWindow, shown: bool) {
    if window.is_visible().ok() != Some(shown) {
        let _ = if shown { window.show() } else { window.hide() };
    }
}

pub fn watch(app: AppHandle) {
    std::thread::spawn(move || loop {
        place(&app);
        std::thread::sleep(std::time::Duration::from_secs(2));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Frame = Frame {
        x: 1920,
        y: -200,
        width: 2560,
        height: 1440,
    };

    #[test]
    fn the_main_window_stays_on_the_preferred_monitor_and_extra_displays_get_stable_labels() {
        assert_eq!(desired_windows(false, 3, 1), vec![("notch".to_string(), 1)]);
        assert_eq!(
            desired_windows(true, 3, 1),
            vec![
                ("notch".to_string(), 1),
                ("notch-1".to_string(), 0),
                ("notch-3".to_string(), 2),
            ]
        );
    }

    #[test]
    fn every_edge_keeps_the_window_inside_its_own_monitor_at_any_size() {
        for edge in ["left", "right", "top", "bottom"] {
            for size in [0.5, 2.0] {
                for scale in [1.0, 1.5] {
                    let frame = window_frame(edge, size, 0.7, SECOND, scale);
                    assert!(
                        frame.x >= SECOND.x && frame.y >= SECOND.y,
                        "{edge} {size} {scale}"
                    );
                    assert!(
                        frame.x + frame.width as i32 <= SECOND.x + SECOND.width as i32
                            && frame.y + frame.height as i32 <= SECOND.y + SECOND.height as i32,
                        "{edge} {size} {scale}"
                    );
                }
            }
        }
        let right = window_frame("right", 1.0, 0.5, SECOND, 1.5);
        assert_eq!((right.x, right.width), (SECOND.x + 2560 - 510, 510));
        let bottom = window_frame("bottom", 1.0, 0.5, SECOND, 2.0);
        assert_eq!((bottom.y, bottom.height), (SECOND.y + 1440 - 920, 920));
    }

    #[test]
    fn dragging_follows_the_edge_axis_of_the_notchs_own_monitor() {
        assert_eq!(along_ratio("top", SECOND, (1920.0 + 640.0, 0.0)), 0.25);
        assert_eq!(along_ratio("bottom", SECOND, (0.0, 0.0)), 0.0);
        assert_eq!(along_ratio("right", SECOND, (0.0, -200.0 + 1080.0)), 0.75);
        assert_eq!(along_ratio("left", SECOND, (0.0, 5000.0)), 1.0);
    }
}
