#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_webview_window("main").ok_or("Missing island window")?;
            // Propagate the native taskbar API result instead of relying solely on
            // the initial window attribute, whose platform setup may ignore errors.
            window.set_skip_taskbar(true)?;
            eprintln!(
                "Isle window ready: taskbar exclusion succeeded; decorated={}; resizable={}; topmost={}; scale={}",
                window.is_decorated()?,
                window.is_resizable()?,
                window.is_always_on_top()?,
                window.scale_factor()?
            );
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Isle could not start");
}
