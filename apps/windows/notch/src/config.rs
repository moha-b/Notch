use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod validation;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub disabled_providers: Vec<String>,
    pub provider_order: Vec<String>,
    pub muted_providers: Vec<String>,
    pub edge: String,
    pub display: Option<String>,
    pub follow_focus: bool,
    /// "main_display" | "all_displays": one notch, or one per monitor (Mac `NotchScreenScope`)
    pub scope: String,
    /// "always_show" | "on_hover" | "hidden" (Mac `NotchVisibility`)
    pub visibility: String,
    pub scale: f64,
    pub accent: String,
    pub reset_format: String,
    pub show_usage_pace: bool,
    pub gemini_monthly_budget: Option<u64>,
    pub announce_completion: bool,
    pub sounds: bool,
    pub peek_seconds: u32,
    pub automatic_checks: bool,
    pub onboarding_complete: bool,
    #[serde(default = "default_port")]
    pub port: u16,
    /// "auto" | "zh" | "en" | "ja" | "ko"
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(default)]
    pub bar_x: Option<i32>,
    #[serde(default)]
    pub bar_y: Option<i32>,
    /// Logical width of the bar (wheel-adjustable, 220-520); None = default 360
    #[serde(default)]
    pub bar_w: Option<u32>,
    /// Allow dragging + wheel resizing (tray toggle, off by default to prevent accidental drags)
    #[serde(default)]
    pub drag_enabled: bool,
    /// Vertical position of the notch: the window centre as a fraction of the primary monitor's height (0 = top, 1 = bottom), default 0.5; saved after a drag
    #[serde(default = "default_notch_y")]
    pub notch_y: f64,
}

fn default_notch_y() -> f64 {
    0.5
}

fn default_port() -> u16 {
    crate::identity::PORT
}
fn default_lang() -> String {
    "auto".into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            disabled_providers: Vec::new(),
            provider_order: crate::providers::FAMILIES
                .iter()
                .map(|id| (*id).into())
                .collect(),
            muted_providers: Vec::new(),
            edge: "right".into(),
            display: None,
            follow_focus: false,
            scope: "main_display".into(),
            visibility: "on_hover".into(),
            scale: 1.0,
            accent: "#00FF88".into(),
            reset_format: "relative".into(),
            show_usage_pace: false,
            gemini_monthly_budget: None,
            announce_completion: true,
            sounds: false,
            peek_seconds: 5,
            automatic_checks: true,
            onboarding_complete: false,
            port: default_port(),
            lang: default_lang(),
            bar_x: None,
            bar_y: None,
            bar_w: None,
            drag_enabled: false,
            notch_y: default_notch_y(),
        }
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(crate::identity::NAMESPACE)
        .join("config.json")
}

pub fn load() -> Result<Config, String> {
    load_from(&config_path())
}

fn load_from(path: &std::path::Path) -> Result<Config, String> {
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(error) => return Err(format!("Could not read Notch settings: {error}")),
    };
    match decode(&bytes) {
        Ok(settings) => Ok(settings),
        Err(_) => {
            let backup = path.with_extension(format!("invalid-{}", crate::providers::now_ms()));
            preserve_damaged_settings(&backup, &bytes)?;
            Ok(Config {
                disabled_providers: crate::providers::FAMILIES
                    .iter()
                    .map(|id| (*id).into())
                    .collect(),
                ..Config::default()
            })
        }
    }
}

fn preserve_damaged_settings(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let mut backup = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("Cannot preserve damaged settings: {error}"))?;
    backup
        .write_all(bytes)
        .map_err(|error| format!("Cannot preserve damaged settings: {error}"))
}

fn decode(bytes: &[u8]) -> Result<Config, String> {
    let settings: Config = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    settings.validate()?;
    Ok(settings)
}

pub fn save(cfg: &Config) {
    if let Err(error) = try_save(cfg) {
        eprintln!("Could not save Notch settings: {error}");
    }
}

pub fn try_save(cfg: &Config) -> Result<(), String> {
    cfg.validate()?;
    let path = config_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(cfg).map_err(|error| error.to_string())?;
    let pending = path.with_extension("pending");
    std::fs::write(&pending, bytes).map_err(|error| error.to_string())?;
    std::fs::rename(pending, path).map_err(|error| error.to_string())
}
