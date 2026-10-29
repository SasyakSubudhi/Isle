use crate::{
    core::{config::FeatureSettings, Core},
    window::WindowManager,
};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub fn wake(app: &AppHandle, tab: &str, focus: bool) {
    if let Some(window) = app.get_webview_window("main") {
        if focus {
            let manager = app.state::<WindowManager>();
            let _ = manager.focusable(&window, true);
            let _ = window.set_ignore_cursor_events(false);
        }
        let _ = app.emit("island://hotkey", tab);
    }
}
pub fn register_hotkeys(app: &AppHandle, settings: &FeatureSettings) -> Vec<String> {
    let mut errors = Vec::new();
    let mut unique = std::collections::HashSet::new();
    for (name, value) in &settings.hotkeys {
        if value
            .parse::<tauri_plugin_global_shortcut::Shortcut>()
            .is_err()
        {
            errors.push(format!("{name}: invalid hotkey {value}"));
        }
        if !unique.insert(value.to_lowercase()) {
            errors.push(format!("{name}: duplicate hotkey"));
        }
    }
    if !errors.is_empty() {
        return errors;
    }
    if let Err(e) = app.global_shortcut().unregister_all() {
        errors.push(e.to_string());
        return errors;
    }
    for (name, value) in &settings.hotkeys {
        let name = name.clone();
        if let Err(error) =
            app.global_shortcut()
                .on_shortcut(value.as_str(), move |app, _, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    if name == "selection" {
                        let app = app.clone();
                        std::thread::spawn(move || {
                            let window = app.get_webview_window("main");
                            if let Some(window) = window {
                                let result =
                                    window.hwnd().map_err(|e| e.to_string()).and_then(|hwnd| {
                                        crate::clipboard::selected_text(hwnd.0 as usize)
                                    });
                                match result {
                                    Ok(text) => {
                                        wake(&app, "dictionary", true);
                                        let _ = app.emit("dictionary://selection", text);
                                    }
                                    Err(error) => {
                                        wake(&app, "dictionary", true);
                                        let _ = app.emit("app://error", error);
                                    }
                                }
                            }
                        });
                    } else {
                        wake(app, &name, true);
                    }
                })
        {
            errors.push(format!("{value}: {error}"));
        }
    }
    errors
}
/// Isle should always be there: turn on start-with-Windows once (the first run
/// of a version that has this), and repair the registry entry if a reinstall
/// or cleanup tool removed it while the setting is still on.
fn ensure_autostart(app: &AppHandle, core: &Core) {
    use tauri_plugin_autostart::ManagerExt;
    const KEY: &str = "autostart_initialized";
    let result = (|| -> Result<(), String> {
        let mut settings = core.settings()?;
        if core.kv_get::<bool>(KEY)? != Some(true) {
            settings.launch_at_startup = true;
            core.save_settings(settings.clone())?;
            core.kv_set(KEY, &true)?;
        }
        let enabled = app.autolaunch().is_enabled().unwrap_or(false);
        if settings.launch_at_startup && !enabled {
            app.autolaunch().enable().map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    if let Err(e) = result {
        eprintln!("Isle autostart: {e}");
    }
}
pub fn start(app: AppHandle, core: Core) -> Result<(), String> {
    ensure_autostart(&app, &core);
    let errors = register_hotkeys(&app, &core.settings()?);
    core.kv_set("hotkey_errors", &errors)?;
    if !errors.is_empty() {
        eprintln!("Isle hotkeys: {}", errors.join("; "));
    } else {
        eprintln!("Isle global hotkeys registered");
    }
    let menu = tauri::menu::MenuBuilder::new(&app)
        .text("app:open", "Open Isle")
        .text("app:settings", "Settings")
        .separator()
        .text("app:pause", "Pause / resume tracking")
        .text("app:privacy", "Toggle presentation mode")
        .text("app:dnd", "Toggle Do Not Disturb")
        .text("app:capture-cancel", "Cancel screenshot capture")
        .separator()
        .text("quit", "Quit Isle")
        .build()
        .map_err(|e| e.to_string())?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("Application icon missing")?;
    tauri::tray::TrayIconBuilder::new()
        .icon(icon)
        .tooltip("Isle · your desktop pocket")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| crate::commands::handle_menu(app, event.id().as_ref()))
        .build(&app)
        .map_err(|e| e.to_string())?;
    crate::clipboard::start(app.clone(), core.clone());
    crate::downloads::start(app.clone(), core.clone());
    crate::usage::start(app.clone(), core.clone());
    crate::context::start(app.clone(), core.clone());
    let media = crate::media::Media::default();
    app.manage(media.clone());
    crate::media::start(app.clone(), media, core.0.cache.clone());
    let activity = crate::activity::Activity::default();
    app.manage(activity.clone());
    crate::activity::start(app.clone(), core.clone(), activity);
    let meter = crate::activity::audio::Meter::default();
    app.manage(meter.clone());
    crate::activity::audio::start(app.clone(), meter);
    let ai = crate::ai_usage::AiState::default();
    app.manage(ai.clone());
    crate::ai_usage::start(app.clone(), core.clone(), ai);
    std::thread::spawn(move || {
        let mut expire = Instant::now();
        loop {
            std::thread::sleep(Duration::from_secs(1));
            if let Ok(Some(timer)) = core.timer_tick() {
                let _ = app.emit("timer://completed", &timer);
            }
            if let Ok(timer) = core.timer() {
                if timer.status == "running" {
                    let _ = app.emit("timer://changed", timer);
                }
            }
            if expire.elapsed() > Duration::from_secs(60) {
                expire = Instant::now();
                if core.shelf_expire().is_ok_and(|count| count > 0) {
                    let _ = app.emit("shelf://changed", ());
                }
            }
        }
    });
    Ok(())
}
