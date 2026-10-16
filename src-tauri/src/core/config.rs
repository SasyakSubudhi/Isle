use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct FeatureSettings {
    pub clipboard_history: bool,
    pub copy_into_cache: bool,
    pub shelf_expiry_hours: u32,
    pub watch_downloads: bool,
    pub downloads_folder: String,
    pub auto_add_downloads: bool,
    pub tracking_paused: bool,
    pub idle_timeout_seconds: u32,
    pub excluded_apps: Vec<String>,
    pub track_window_titles: bool,
    pub app_limits: BTreeMap<String, u32>,
    pub chrome_executable: String,
    pub chrome_profile: String,
    pub gmail_url: String,
    pub outlook_url: String,
    pub whatsapp_url: String,
    pub focus_minutes: u32,
    pub short_break_minutes: u32,
    pub long_break_minutes: u32,
    pub presentation_mode: bool,
    pub theme: String,
    pub reduced_motion: bool,
    pub hover_expand: bool,
    pub launch_at_startup: bool,
    pub hotkeys: BTreeMap<String, String>,
}
impl Default for FeatureSettings {
    fn default() -> Self {
        Self {
            clipboard_history: false,
            copy_into_cache: false,
            shelf_expiry_hours: 24,
            watch_downloads: true,
            downloads_folder: crate::platform::downloads_folder().to_string_lossy().into(),
            auto_add_downloads: false,
            tracking_paused: false,
            idle_timeout_seconds: 120,
            excluded_apps: Vec::new(),
            track_window_titles: false,
            app_limits: BTreeMap::new(),
            chrome_executable: String::new(),
            chrome_profile: String::new(),
            gmail_url: "https://mail.google.com/mail/u/0/".into(),
            outlook_url: "https://outlook.live.com/mail/0/".into(),
            whatsapp_url: "https://web.whatsapp.com/".into(),
            focus_minutes: 25,
            short_break_minutes: 5,
            long_break_minutes: 15,
            presentation_mode: false,
            theme: "system".into(),
            reduced_motion: false,
            hover_expand: true,
            launch_at_startup: false,
            hotkeys: [
                ("toggle", "Ctrl+Alt+Space"),
                ("dictionary", "Ctrl+Alt+D"),
                ("shelf", "Ctrl+Alt+S"),
                ("usage", "Ctrl+Alt+T"),
                ("selection", "Ctrl+Alt+Shift+D"),
            ]
            .into_iter()
            .map(|(a, b)| (a.into(), b.into()))
            .collect(),
        }
    }
}
impl FeatureSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.app_limits.values().any(|minutes| *minutes > 1440) {
            return Err("Daily app limits must be 0–1440 minutes".into());
        }
        if !(1..=720).contains(&self.shelf_expiry_hours)
            || !(10..=3600).contains(&self.idle_timeout_seconds)
        {
            return Err("Expiry must be 1–720 hours and idle timeout 10–3600 seconds".into());
        }
        if ![
            self.focus_minutes,
            self.short_break_minutes,
            self.long_break_minutes,
        ]
        .iter()
        .all(|n| (1..=180).contains(n))
        {
            return Err("Timer durations must be 1–180 minutes".into());
        }
        if !["dark", "light", "system"].contains(&self.theme.as_str()) {
            return Err("Unknown theme".into());
        }
        for url in [&self.gmail_url, &self.outlook_url, &self.whatsapp_url] {
            let parsed = reqwest::Url::parse(url).map_err(|_| "Shortcut URL is invalid")?;
            if parsed.scheme() != "https" {
                return Err("Chrome shortcuts require HTTPS URLs".into());
            }
        }
        Ok(())
    }
}
pub fn load() -> Result<FeatureSettings, String> {
    let path = crate::settings::path()?;
    if !path.exists() {
        return Ok(FeatureSettings::default());
    }
    let data: serde_json::Value =
        serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let settings = data
        .get("features")
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    FeatureSettings::validate(&settings)?;
    Ok(settings)
}
pub fn save(settings: &FeatureSettings) -> Result<(), String> {
    let _guard = crate::settings::CONFIG_LOCK
        .lock()
        .map_err(|_| "Settings busy")?;
    let path = crate::settings::path()?;
    let mut value = if path.exists() {
        serde_json::from_slice::<serde_json::Value>(&fs::read(&path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?
    } else {
        serde_json::json!({})
    };
    value
        .as_object_mut()
        .ok_or("Settings must be an object")?
        .insert(
            "features".into(),
            serde_json::to_value(settings).map_err(|e| e.to_string())?,
        );
    fs::create_dir_all(path.parent().ok_or("Invalid settings path")?).map_err(|e| e.to_string())?;
    let temp = path.with_extension("json.tmp");
    fs::write(
        &temp,
        serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::rename(temp, path).map_err(|e| e.to_string())
}
