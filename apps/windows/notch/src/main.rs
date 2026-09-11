#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

#[path = "../../identity.rs"]
mod identity;
mod autostart;
mod config;
mod doctor;
mod focus;
mod hooks_install;
mod i18n;
mod server;
mod state;
mod tray;
mod usage;
#[cfg(test)]
mod usage_tests;
mod codex;
mod cursor;
mod antigravity;
mod glyphs;
mod activity;
mod diag;
mod watcher;
mod providers;
mod settings;
mod placement;
mod updates;
mod sounds;
mod hit_regions;

use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

/// Logical size of the notch window: the 70 pt pill column on the right plus room for the hover card on the left.
pub const NOTCH_W: f64 = 340.0;
/// Hand-bumped build tag, written to run.log at startup so a log can always be matched to the exe that wrote it.
pub const BUILD: &str = "r31";
pub const NOTCH_H: f64 = 460.0; // 300 clipped the card once it held three window blocks plus the session list

pub struct AppState {
    pub profiles: Vec<providers::profiles::Profile>,
    pub providers: Mutex<std::collections::BTreeMap<String, usage::UsageSnapshot>>,
    pub store: Mutex<state::Store>,
    pub cfg: Mutex<config::Config>,
    pub usage: Mutex<usage::UsageSnapshot>,
    /// Codex snapshot (same UsageSnapshot shape; status may also be none/absent)
    pub codex: Mutex<usage::UsageSnapshot>,
    pub cursor: Mutex<usage::UsageSnapshot>,
    pub antigravity: Mutex<usage::UsageSnapshot>,
    /// Provider glyph cache, collected at launch and again on a tray refresh
    pub glyphs: Mutex<std::collections::HashMap<String, glyphs::Glyph>>,
    /// Working state of the non-Claude providers (Cursor reports it; Codex and Antigravity are inferred from recent writes)
    pub activity: Mutex<Vec<activity::Activity>>,
}

fn resolved_lang(raw: &str) -> String {
    if raw == "auto" {
        i18n::resolve_auto().to_string()
    } else {
        raw.to_string()
    }
}

pub fn broadcast(app: &AppHandle) {
    let st = app.state::<AppState>();
    let snap = {
        let store = st.store.lock().unwrap();
        let cfg = st.cfg.lock().unwrap();
        store.snapshot(&cfg.lang, &resolved_lang(&cfg.lang), false)
    };
    let _ = app.emit("state", &snap);
}

pub fn place_notch(app: &AppHandle) { placement::place(app); }

/// Older entry point name still used by tray.rs
pub fn reset_bar(app: &AppHandle) {
    {
        let st = app.state::<AppState>();
        let mut c = st.cfg.lock().unwrap();
        c.notch_y = 0.5;
        config::save(&c);
    }
    place_notch(app);
}

/// Drag along the notch's edge. The page calls this once after a press on the pill moves more than
/// 4 px; from then on a Rust thread follows the system cursor (WebView mousemove is unreliable
/// once dragging starts). The window already spans its monitor's whole edge, so the drag moves the
/// pill inside it: the pointer's position along the edge of that window's own monitor becomes the
/// shared offset, every display's notch follows it live, and releasing the left button saves it.
static DRAGGING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(windows)]
fn left_button_down() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
    unsafe { (GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16 & 0x8000) != 0 }
}
#[cfg(not(windows))]
fn left_button_down() -> bool {
    false
}

#[tauri::command]
fn drag_begin(app: AppHandle, window: tauri::WebviewWindow) {
    if DRAGGING.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        let Some(monitor) = placement::current_frame(&window) else {
            DRAGGING.store(false, std::sync::atomic::Ordering::SeqCst);
            return;
        };
        let edge = app.state::<AppState>().cfg.lock().unwrap().edge.clone();
        let mut last: Option<f64> = None;
        while left_button_down() {
            if let Ok(cur) = app.cursor_position() {
                let ratio = placement::along_ratio(&edge, monitor, (cur.x, cur.y));
                if last.is_none_or(|previous| (previous - ratio).abs() > 0.002) {
                    last = Some(ratio);
                    let settings = {
                        let st = app.state::<AppState>();
                        let mut c = st.cfg.lock().unwrap();
                        c.notch_y = ratio;
                        c.clone()
                    };
                    let _ = app.emit("settings", &settings);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
        if let Some(ratio) = last {
            let st = app.state::<AppState>();
            config::save(&st.cfg.lock().unwrap());
            applog(&format!("notch drag: edge={edge} ratio={ratio:.3}"));
        }
        DRAGGING.store(false, std::sync::atomic::Ordering::SeqCst);
        let _ = app.emit("drag_end", last.is_some());
    });
}
pub fn place_bar(app: &AppHandle) {
    place_notch(app);
}
pub fn toggle_drag(app: &AppHandle) {
    // The notch stays welded to the edge; kept as a no-op for the tray menu code path
    let _ = app;
}

pub fn apply_lang(app: &AppHandle, lang: &str) {
    {
        let st = app.state::<AppState>();
        let mut c = st.cfg.lock().unwrap();
        c.lang = lang.to_string();
        config::save(&c);
    }
    if let Some(tray) = app.tray_by_id("main") {
        if let Ok(menu) = tray::build_menu(app, lang) {
            let _ = tray.set_menu(Some(menu));
        }
    }
    broadcast(app);
}

/// A notch window must never take focus: WS_EX_NOACTIVATE + WS_EX_TOOLWINDOW
#[cfg(windows)]
pub fn noactivate(w: &tauri::WebviewWindow) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };
    if let Ok(h) = w.hwnd() {
        unsafe {
            let hwnd = windows::Win32::Foundation::HWND(h.0 as isize as *mut core::ffi::c_void);
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(
                hwnd,
                GWL_EXSTYLE,
                ex | WS_EX_NOACTIVATE.0 as isize | WS_EX_TOOLWINDOW.0 as isize,
            );
        }
    }
}
#[cfg(not(windows))]
pub fn noactivate(_w: &tauri::WebviewWindow) {}

// ---------------- commands ----------------

#[tauri::command]
fn get_state(state: tauri::State<AppState>) -> state::Snapshot {
    let store = state.store.lock().unwrap();
    let cfg = state.cfg.lock().unwrap();
    store.snapshot(&cfg.lang, &resolved_lang(&cfg.lang), false)
}

#[tauri::command]
fn get_usage(state: tauri::State<AppState>) -> usage::UsageSnapshot {
    state.usage.lock().unwrap().clone()
}

#[tauri::command]
fn refresh_usage() {
    usage::request_refresh();
    codex::request_refresh();
    cursor::request_refresh();
    antigravity::request_refresh();
}

#[tauri::command]
fn get_antigravity(state: tauri::State<AppState>) -> usage::UsageSnapshot {
    state.antigravity.lock().unwrap().clone()
}

#[tauri::command]
fn get_activity(state: tauri::State<AppState>) -> Vec<activity::Activity> {
    state.activity.lock().unwrap().clone()
}

#[tauri::command]
fn get_glyphs(state: tauri::State<AppState>) -> std::collections::HashMap<String, glyphs::Glyph> {
    state.glyphs.lock().unwrap().clone()
}

/// Collects the glyphs again and pushes them to the page (tray refresh, or the user just dropped in an override)
pub fn reload_glyphs(app: &AppHandle) {
    let m = glyphs::collect();
    let st = app.state::<AppState>();
    *st.glyphs.lock().unwrap() = m.clone();
    let _ = app.emit("glyphs", &m);
}

#[tauri::command]
fn open_data_dir() {
    let dir = config::config_path().parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let _ = std::fs::create_dir_all(glyphs::user_dir());
    let mut cmd = std::process::Command::new("explorer");
    cmd.arg(dir.as_os_str());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let _ = cmd.spawn();
}

#[tauri::command]
fn get_cursor(state: tauri::State<AppState>) -> usage::UsageSnapshot {
    state.cursor.lock().unwrap().clone()
}

#[tauri::command]
fn get_codex(state: tauri::State<AppState>) -> usage::UsageSnapshot {
    state.codex.lock().unwrap().clone()
}

/// A click on a cell opens that provider's usage page
#[tauri::command]
fn open_provider_page(provider: String) {
    let url = match provider.as_str() {
        "codex" => "https://chatgpt.com/#settings/Account",
        "cursor" => "https://cursor.com/dashboard",
        "gemini" | "antigravity" => "https://antigravity.google",
        "glm" => "https://z.ai/manage-apikey/apikey-list",
        "grok" => "https://grok.com",
        "opencode" => "https://opencode.ai",
        "commandcode" => "https://commandcode.ai",
        "copilot" => "https://github.com/settings/copilot",
        "ollama" => "https://ollama.com/settings",
        "ollama-local" | "gemini-api" => return,
        _ => "https://claude.ai/settings/usage",
    };
    let mut cmd = std::process::Command::new("cmd");
    cmd.args(["/C", "start", "", url]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let _ = cmd.spawn();
}

/// Card expansion state per notch window label: hot rectangles in **physical pixels** relative to
/// that window's top-left as x,y,w,h, present only while its card is expanded. The page converts the
/// rectangles with its own devicePixelRatio before reporting them, so no scale conversion happens
/// on this side — WebView2's DPR and the window's scale_factor can disagree (see report_dpr).
static HOT: Mutex<std::collections::BTreeMap<String, Vec<[f64; 4]>>> =
    Mutex::new(std::collections::BTreeMap::new());

#[tauri::command]
fn set_expanded(window: tauri::WebviewWindow, on: bool, rects: Option<Vec<[f64; 4]>>) {
    let mut hot = HOT.lock().unwrap();
    if on {
        hot.insert(window.label().to_string(), rects.unwrap_or_default());
    } else {
        hot.remove(window.label());
    }
}

/// Per notch window: the WebView zoom currently applied (1.0 = uncorrected) and how many
/// corrections it has had
static ZOOM: Mutex<std::collections::BTreeMap<String, (f64, u32)>> =
    Mutex::new(std::collections::BTreeMap::new());

pub fn applog(line: &str) {
    use std::io::Write;
    let log = config::config_path().with_file_name("run.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log) {
        let _ = writeln!(f, "{line}");
    }
}

/// Root cause: with two monitors (150 % / 200 %) WebView2 picked a devicePixelRatio of 2.0 while
/// the window was sized for the primary monitor's 1.5, so the page was 255 CSS px wide instead of
/// the designed 340 and every coordinate conversion was off (the watchdog misfired and the card
/// flashed away). Fix: the page reports its DPR, and when it differs from the scale of the monitor
/// the window is actually on, set_zoom pulls the effective DPR back to that scale, restoring the
/// 340 px width. Each display's notch window keeps its own zoom.
#[tauri::command]
fn report_dpr(window: tauri::WebviewWindow, dpr: f64, w: f64, h: f64) {
    let want = window
        .current_monitor()
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or_else(|| window.scale_factor().unwrap_or(1.0));
    let mut zooms = ZOOM.lock().unwrap();
    let (z, corrections) = zooms.entry(window.label().to_string()).or_insert((1.0, 0));
    let base = if *z > 0.0 { dpr / *z } else { dpr };
    let target = if base > 0.0 { want / base } else { 1.0 };
    applog(&format!(
        "dpr report {}: dpr={dpr:.3} viewport={w:.0}x{h:.0} monitor_scale={want:.3} zoom_applied={:.3} -> target_zoom={target:.3}",
        window.label(),
        *z
    ));
    // Oscillation guard: at most three corrections per window (if the DPR does not follow the zoom, stop chasing it)
    if (dpr - want).abs() > 0.02
        && (target - *z).abs() > 0.01
        && (0.25..=4.0).contains(&target)
        && *corrections < 3
    {
        *corrections += 1;
        match window.set_zoom(target) {
            Ok(()) => {
                *z = target;
                applog(&format!("dpr correction: set_zoom({target:.3}) ok"));
            }
            Err(e) => applog(&format!("dpr correction failed: {e}")),
        }
    }
}

/// WebView2's mouseleave is unreliable inside a NOACTIVATE transparent window — a cursor that
/// leaves quickly often produces no WM_MOUSELEAVE, and the card stays up. Rather than trust DOM
/// events, the Rust side watches the system cursor while the card is expanded and emits
/// pointer_left once the cursor is outside; the page collapses after its 250 ms grace period.
/// "Outside the window" is not the test, though: the window has a 340×460 transparent area, so
/// the cursor is compared against the hot rectangles the page reports (pill, card, and the gap
/// between them), and two consecutive misses (300 ms) count as leaving.
fn start_pointer_watchdog(app: AppHandle) {
    std::thread::spawn(move || {
        // Each display's notch is watched on its own; only the window whose card is open is told the pointer left
        let mut misses: std::collections::BTreeMap<String, u8> = Default::default();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(150));
            let expanded = HOT.lock().unwrap().clone();
            misses.retain(|label, _| expanded.contains_key(label));
            let Ok(cur) = app.cursor_position() else { continue };
            for (label, rects) in expanded {
                let Some(w) = app.get_webview_window(&label) else { continue };
                let Ok(pos) = w.outer_position() else { continue };
                // Cursor position relative to the window's top-left, in physical pixels; the hot rectangles are physical too, so no scale conversion
                let lx = cur.x - pos.x as f64;
                let ly = cur.y - pos.y as f64;
                const PAD: f64 = 10.0;
                let in_window = w
                    .outer_size()
                    .map(|s| lx >= 0.0 && ly >= 0.0 && lx < s.width as f64 && ly < s.height as f64)
                    .unwrap_or(true);
                let mut inside = in_window && rects.iter().any(|r| {
                    lx >= r[0] - PAD && ly >= r[1] - PAD && lx < r[0] + r[2] + PAD && ly < r[1] + r[3] + PAD
                });
                // The gap between hot rectangles (pill and card) counts as inside: use the bounding box of all of them
                if !inside && in_window && rects.len() > 1 {
                    let x0 = rects.iter().map(|r| r[0]).fold(f64::MAX, f64::min);
                    let y0 = rects.iter().map(|r| r[1]).fold(f64::MAX, f64::min);
                    let x1 = rects.iter().map(|r| r[0] + r[2]).fold(f64::MIN, f64::max);
                    let y1 = rects.iter().map(|r| r[1] + r[3]).fold(f64::MIN, f64::max);
                    inside = lx >= x0 && ly >= y0 && lx < x1 && ly < y1;
                }
                static LOGGED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
                if LOGGED.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 12 {
                    applog(&format!(
                        "watchdog {label}: cursor_rel=({lx:.0},{ly:.0}) inside={inside} rects={rects:?} winpos=({},{})",
                        pos.x, pos.y
                    ));
                }
                let miss = misses.entry(label.clone()).or_insert(0);
                if inside {
                    *miss = 0;
                } else {
                    *miss += 1;
                    if *miss >= 2 {
                        *miss = 0;
                        HOT.lock().unwrap().remove(&label);
                        let _ = app.emit("pointer_left", &label);
                    }
                }
            }
        }
    });
}

/// Log channel for the page: JS writes key diagnostics into run.log (if invoke itself fails, the page reports on screen instead)
#[tauri::command]
fn log_js(msg: String) {
    applog(&format!("js: {}", msg.chars().take(600).collect::<String>()));
}

#[tauri::command]
fn open_usage_page() {
    let mut cmd = std::process::Command::new("cmd");
    cmd.args(["/C", "start", "", "https://claude.ai/settings/usage"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let _ = cmd.spawn();
}

#[tauri::command]
fn focus_session(app: AppHandle, id: String) -> bool {
    let ppid = {
        let st = app.state::<AppState>();
        let store = st.store.lock().unwrap();
        store.ppid_of(&id)
    };
    match ppid {
        Some(p) => focus::focus_terminal(p),
        None => focus::focus_claude_desktop(),
    }
}

#[tauri::command]
fn dismiss_session(app: AppHandle, id: String) {
    {
        let st = app.state::<AppState>();
        let mut store = st.store.lock().unwrap();
        store.dismiss(&id);
    }
    broadcast(&app);
}

#[tauri::command]
fn set_lang(app: AppHandle, lang: String) {
    apply_lang(&app, &lang);
}

/// Seen-clears-it: looking at a session acknowledges it (engine behaviour, unchanged)
#[cfg(windows)]
fn ack_scan(app: &AppHandle) -> bool {
    let need = {
        let st = app.state::<AppState>();
        let store = st.store.lock().unwrap();
        store.has_done()
    };
    if !need {
        return false;
    }
    let fg = focus::fg_pid();
    if fg == 0 {
        return false;
    }
    let maps = focus::proc_maps();
    let fg_name = maps.name.get(&fg).cloned().unwrap_or_default();
    let fg_is_claude_desktop = fg_name.contains("claude") && !fg_name.contains("notch");
    let st = app.state::<AppState>();
    let mut store = st.store.lock().unwrap();
    store.ack_done(|s| {
        if s.ppid == 0 {
            fg_is_claude_desktop
        } else {
            focus::pid_hits_chain(fg, &focus::chain_of(s.ppid, &maps.ppid), &maps)
        }
    })
}
#[cfg(not(windows))]
fn ack_scan(_app: &AppHandle) -> bool {
    false
}

// ---------------- main ----------------

#[cfg(windows)]
fn attach_console() {
    use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}
#[cfg(not(windows))]
fn attach_console() {}

fn report(r: Result<String, String>) {
    let failed = r.is_err();
    let msg = match r {
        Ok(m) => format!("OK: {m}"),
        Err(e) => format!("FAILED: {e}"),
    };
    println!("{msg}");
    let log = config::config_path().with_file_name("install.log");
    if let Some(parent) = log.parent() { let _ = std::fs::create_dir_all(parent); }
    let _ = std::fs::write(log, &msg);
    if failed { std::process::exit(1); }
}

fn main() {
    attach_console();
    let args: Vec<String> = std::env::args().collect();
    if let Some(cmd) = args.get(1) {
        match cmd.as_str() {
            "uninstall-installed-hooks" => {
                let result = hooks_install::uninstall_current_installation();
                let code = if result.is_ok() { 0 } else { 1 };
                report(result);
                std::process::exit(code);
            }
            "install-hooks" => {
                report(hooks_install::install());
                return;
            }
            "uninstall-hooks" => {
                report(hooks_install::uninstall());
                return;
            }
            "autostart" => {
                let r = match args.get(2).map(|s| s.as_str()) {
                    Some("on") => autostart::enable(),
                    Some("off") => autostart::disable(),
                    _ => Err("usage: notch.exe autostart on|off".into()),
                };
                report(r);
                return;
            }
            "doctor" => {
                let out = if args.get(2).map(|s| s.as_str()) == Some("deep") { diag::run() } else { doctor::run() };
                println!("{out}");
                let log = config::config_path().with_file_name("doctor.log");
                let _ = std::fs::write(log, &out);
                return;
            }
            _ => {}
        }
    }

    let mut cfg = match config::load() {
        Ok(settings) => settings,
        Err(error) => { report(Err(error)); return; }
    };
    let profiles = match providers::profiles::discover() {
        Ok(profiles) => profiles,
        Err(_) => { report(Err("Cannot discover provider profiles. Check access to your home directory.".into())); return; }
    };
    providers::profiles::reconcile(&mut cfg, &profiles);
    let port = cfg.port;

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Launching a freshly built exe while the old one is still running lands here: the new
            // instance is turned away and what stays on screen is the old process. Say so loudly.
            applog(&format!("single instance: another launch was refused; the running instance is build={BUILD} — quit it from the tray first if you just rebuilt"));
            let _ = app.emit("notice", format!("Notch is already running ({BUILD}) — quit it from the tray before starting a new build"));
        }))
        .manage(updates::Updates::default())
        .manage(AppState {
            profiles,
            providers: Mutex::new(providers::load_cache()),
            store: Mutex::new(Default::default()),
            cfg: Mutex::new(cfg),
            usage: Mutex::new(usage::load_persisted()),
            codex: Mutex::new(codex::load_persisted()),
            cursor: Mutex::new(cursor::load_persisted()),
            antigravity: Mutex::new(antigravity::load_persisted()),
            glyphs: Mutex::new(Default::default()),
            activity: Mutex::new(Vec::new()),
        })
        .invoke_handler(tauri::generate_handler![
            hit_regions::set_hit_regions,
            sounds::play_session_sound,
            updates::update_status, updates::check_update, updates::download_update, updates::install_update,
            settings::get_settings, settings::save_settings, settings::get_providers, settings::get_profile_names,
            settings::open_settings, settings::get_displays, settings::save_ollama_key, settings::delete_ollama_key,
            get_state,
            get_usage,
            get_codex,
            get_cursor,
            get_antigravity,
            get_glyphs,
            get_activity,
            open_data_dir,
            drag_begin,
            open_provider_page,
            refresh_usage,
            open_usage_page,
            set_expanded,
            report_dpr,
            log_js,
            focus_session,
            dismiss_session,
            set_lang
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            if identity::STABLE { handle.plugin(tauri_plugin_updater::Builder::new().build())?; }
            updates::start(handle.clone());
            if !handle.state::<AppState>().cfg.lock().unwrap().onboarding_complete {
                settings::open_settings(handle.clone()).map_err(std::io::Error::other)?;
            }
            // The configured main window gets the notch style first; placement then opens any other
            // display's window and shows them all unless the notch is hidden
            let main_window = handle
                .get_webview_window(placement::MAIN_WINDOW)
                .ok_or_else(|| std::io::Error::other("The notch window is missing."))?;
            hit_regions::clear(&main_window).map_err(std::io::Error::other)?;
            noactivate(&main_window);
            place_notch(&handle);
            placement::watch(handle.clone());
            tray::setup(&handle)?;
            server::start(handle.clone(), port);
            watcher::start(handle.clone());
            providers::start(handle.clone());
            usage::start(handle.clone());
            codex::start(handle.clone());
            cursor::start(handle.clone());
            antigravity::start(handle.clone());
            activity::start(handle.clone());
            // Collecting glyphs may read icon resources out of a few executables; do it off the main thread and push when done
            let gh = handle.clone();
            std::thread::spawn(move || reload_glyphs(&gh));
            start_pointer_watchdog(handle.clone());
            // Seen-clears-it scan
            let acker = handle.clone();
            std::thread::spawn(move || {
                activity::lower_thread_priority();
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(1500));
                    if ack_scan(&acker) {
                        broadcast(&acker);
                    }
                }
            });
            // Stale session cleanup
            let sweeper = handle.clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(30));
                let changed = {
                    let st = sweeper.state::<AppState>();
                    let mut s = st.store.lock().unwrap();
                    s.sweep()
                };
                if changed {
                    broadcast(&sweeper);
                }
            });
            // Persist the config (notch-hook reads the port from it)
            {
                let st = handle.state::<AppState>();
                let c = st.cfg.lock().unwrap();
                config::save(&c);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Notch failed to start");
}
