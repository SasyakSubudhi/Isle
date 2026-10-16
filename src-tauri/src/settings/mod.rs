use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
pub static CONFIG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum PositionMode {
    #[default]
    Taskbar,
    TopCenter,
    Custom,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct WindowPreferences {
    pub position_mode: PositionMode,
    pub monitor: Option<String>,
    pub pill_width: f64,
    pub custom_x: f64,
    pub custom_y: f64,
}

impl Default for WindowPreferences {
    fn default() -> Self {
        Self {
            position_mode: PositionMode::Taskbar,
            monitor: None,
            pill_width: 148.0,
            custom_x: 280.0,
            custom_y: 36.0,
        }
    }
}

impl WindowPreferences {
    pub fn validate(&self) -> Result<(), String> {
        if !self.pill_width.is_finite() || !(120.0..=240.0).contains(&self.pill_width) {
            return Err("Pill width must be 120–240 logical pixels".into());
        }
        if !self.custom_x.is_finite() || !self.custom_y.is_finite() {
            return Err("Custom coordinates must be finite".into());
        }
        Ok(())
    }
}

pub fn path() -> Result<PathBuf, String> {
    std::env::var_os("APPDATA")
        .map(|p| PathBuf::from(p).join("Isle/settings.json"))
        .ok_or("Windows APPDATA is unavailable".into())
}

pub fn load(path: &Path) -> Result<WindowPreferences, String> {
    if !path.exists() {
        return Ok(WindowPreferences::default());
    }
    let prefs: WindowPreferences =
        serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    prefs.validate()?;
    Ok(prefs)
}

pub fn save(path: &Path, prefs: &WindowPreferences) -> Result<(), String> {
    let _guard = CONFIG_LOCK.lock().map_err(|_| "Settings busy")?;
    prefs.validate()?;
    // Preserve settings belonging to later modules instead of replacing the file.
    let mut value = if path.exists() {
        serde_json::from_slice::<serde_json::Value>(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?
    } else {
        serde_json::json!({})
    };
    let object = value
        .as_object_mut()
        .ok_or("Settings file must contain a JSON object")?;
    for (key, value) in serde_json::to_value(prefs)
        .map_err(|e| e.to_string())?
        .as_object()
        .ok_or("Invalid preferences")?
    {
        object.insert(key.clone(), value.clone());
    }
    fs::create_dir_all(path.parent().ok_or("Missing settings directory")?)
        .map_err(|e| e.to_string())?;
    let temporary = path.with_extension("json.tmp");
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::rename(temporary, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_non_finite_or_out_of_bounds_values() {
        for width in [f64::NAN, f64::INFINITY, 0.0, 241.0] {
            assert!(WindowPreferences {
                pill_width: width,
                ..Default::default()
            }
            .validate()
            .is_err());
        }
    }
}
