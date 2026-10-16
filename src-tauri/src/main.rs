#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

mod clipboard;
mod commands;
mod context;
mod core;
mod dock;
mod downloads;
mod drop_target;
mod media;
mod platform;
mod services;
mod settings;
mod taskbar;
mod usage;
mod widgets;
mod window;

/// Keeps one Isle running. Two copies fight over the taskbar slot, so a
/// second launch tells the user where the first one is and exits.
#[cfg(windows)]
fn ensure_single_instance() {
    use windows::core::w;
    use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};
    // The handle is intentionally leaked so the mutex lives as long as the process.
    let created =
        unsafe { CreateMutexW(None, true, w!("Local\\app.isle.desktop.single-instance")) };
    if created.is_ok() && unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        eprintln!("Isle is already running; exiting this copy");
        unsafe {
            MessageBoxW(
                None,
                w!("Isle is already running. Right-click the pill and choose Quit Isle before starting another copy."),
                w!("Isle"),
                MB_OK | MB_ICONINFORMATION,
            );
        }
        std::process::exit(0);
    }
}

fn main() {
    #[cfg(windows)]
    ensure_single_instance();
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
            commands::bootstrap,commands::shelf_list,commands::shelf_add_files,commands::shelf_add_text,commands::shelf_paste,commands::shelf_remove,commands::shelf_update,commands::shelf_copy,commands::shelf_start_drag,dock::dock_list,dock::dock_add,dock::dock_remove,dock::dock_clear,dock::dock_start_drag,commands::shelf_open,commands::shelf_clean,
            commands::stacks_list,commands::stack_add,commands::stack_remove,commands::notes_list,commands::note_save,commands::note_remove,commands::dictionary_lookup,commands::dictionary_save,commands::usage_summary,commands::usage_clear,commands::usage_snooze,commands::timer_action,commands::media_command,commands::chrome_open,commands::settings_set,commands::settings_patch,commands::download_action,commands::capture_start,commands::app_quit,
            context::context_current,widgets::habits_list,widgets::habits_save,widgets::weather_get])
        .on_menu_event(|app,event| commands::handle_menu(app,event.id().as_ref()))
        .run(tauri::generate_context!())
        .expect("Isle could not start");
}
