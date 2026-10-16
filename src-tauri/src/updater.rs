//! Automatic updates from GitHub Releases. Updates are signed: the public key
//! is baked in at build time from `ISLE_UPDATER_PUBKEY`, and a build without it
//! reports that updates aren't set up instead of trusting unsigned files.
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

pub const PUBKEY: Option<&str> = option_env!("ISLE_UPDATER_PUBKEY");
pub const NOT_SET_UP: &str =
    "Automatic updates aren't set up in this build yet. Add the updater key to turn them on.";

pub fn plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R, tauri_plugin_updater::Config> {
    let builder = tauri_plugin_updater::Builder::new();
    match PUBKEY {
        Some(key) if !key.trim().is_empty() => builder.pubkey(key),
        _ => builder,
    }
    .build()
}

#[derive(Serialize)]
pub struct UpdateStatus {
    pub current: String,
    pub available: Option<String>,
    pub notes: Option<String>,
}

fn configured() -> Result<(), String> {
    match PUBKEY {
        Some(key) if !key.trim().is_empty() => Ok(()),
        _ => Err(NOT_SET_UP.into()),
    }
}

#[tauri::command]
pub async fn update_check(app: AppHandle) -> Result<UpdateStatus, String> {
    configured()?;
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    Ok(UpdateStatus {
        current: app.package_info().version.to_string(),
        available: update.as_ref().map(|u| u.version.clone()),
        notes: update.and_then(|u| u.body),
    })
}

#[tauri::command]
pub async fn update_install(app: AppHandle) -> Result<(), String> {
    configured()?;
    let Some(update) = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?
    else {
        return Err("Isle is already up to date".into());
    };
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    app.restart();
}
