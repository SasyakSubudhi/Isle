#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

mod clipboard;
mod commands;
mod core;
mod downloads;
mod drop_target;
mod media;
mod platform;
mod services;
mod settings;
mod taskbar;
mod usage;
mod window;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent,None))
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
            let manager = window::start(app.handle().clone())?;
            app.manage(manager);
            let core=core::Core::open()?;
            app.asset_protocol_scope().allow_directory(&core.0.cache,true)?;
            app.manage(core.clone());
            drop_target::install(app.handle().clone(), &window, core.clone())?;
            services::start(app.handle().clone(),core)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::get_taskbar_slot,commands::report_hit_region,commands::set_focusable,commands::position_get,commands::position_set,commands::start_custom_drag,commands::open_position_menu,
            commands::bootstrap,commands::shelf_list,commands::shelf_add_files,commands::shelf_add_text,commands::shelf_paste,commands::shelf_remove,commands::shelf_update,commands::shelf_copy,commands::shelf_start_drag,commands::shelf_open,commands::shelf_clean,
            commands::stacks_list,commands::stack_add,commands::stack_remove,commands::notes_list,commands::note_save,commands::note_remove,commands::dictionary_lookup,commands::dictionary_save,commands::usage_summary,commands::usage_clear,commands::usage_snooze,commands::timer_action,commands::media_command,commands::chrome_open,commands::settings_set,commands::download_action,commands::capture_start])
        .on_menu_event(|app,event| commands::handle_menu(app,event.id().as_ref()))
        .run(tauri::generate_context!())
        .expect("Isle could not start");
}
