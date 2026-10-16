use crate::{
    settings::{PositionMode, WindowPreferences},
    window::{HitRegion, Slot, WindowManager},
};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    AppHandle, Manager, State, WebviewWindow,
};

#[tauri::command]
pub fn get_taskbar_slot(manager: State<'_, WindowManager>) -> Result<Option<Slot>, String> {
    manager.slot()
}
#[tauri::command]
pub fn report_hit_region(
    manager: State<'_, WindowManager>,
    region: HitRegion,
) -> Result<(), String> {
    manager.hit_region(region)
}
#[tauri::command]
pub fn set_focusable(
    window: WebviewWindow,
    manager: State<'_, WindowManager>,
    enabled: bool,
) -> Result<(), String> {
    manager.focusable(&window, enabled)
}
#[tauri::command]
pub fn position_get(manager: State<'_, WindowManager>) -> Result<WindowPreferences, String> {
    manager.preferences()
}
#[tauri::command]
pub fn position_set(
    manager: State<'_, WindowManager>,
    preferences: WindowPreferences,
) -> Result<(), String> {
    manager.save_preferences(preferences)
}
#[tauri::command]
pub fn start_custom_drag(
    window: WebviewWindow,
    manager: State<'_, WindowManager>,
) -> Result<(), String> {
    manager.begin_drag(&window)
}

#[tauri::command]
pub fn open_position_menu(
    app: AppHandle,
    window: WebviewWindow,
    manager: State<'_, WindowManager>,
) -> Result<(), String> {
    let prefs = manager.preferences()?;
    let build = || -> tauri::Result<Menu<tauri::Wry>> {
        let menu = Menu::new(&app)?;
        for (id, label, mode) in [
            (
                "position:taskbar",
                "Taskbar free space",
                PositionMode::Taskbar,
            ),
            ("position:top-center", "Top-center", PositionMode::TopCenter),
            ("position:custom", "Custom position", PositionMode::Custom),
        ] {
            menu.append(&CheckMenuItem::with_id(
                &app,
                id,
                label,
                true,
                prefs.position_mode == mode,
                None::<&str>,
            )?)?;
        }
        menu.append(&MenuItem::with_id(
            &app,
            "drag-help",
            "Hold Alt and drag the pill to place it",
            false,
            None::<&str>,
        )?)?;
        if let Some(reason) = manager.slot().ok().flatten().and_then(|s| s.reason) {
            menu.append(&MenuItem::with_id(
                &app,
                "placement-reason",
                reason.replace('&', "&&"),
                false,
                None::<&str>,
            )?)?;
        }
        menu.append(&PredefinedMenuItem::separator(&app)?)?;
        menu.append(&CheckMenuItem::with_id(
            &app,
            "monitor:primary",
            "Primary monitor",
            true,
            prefs.monitor.is_none(),
            None::<&str>,
        )?)?;
        for (index, monitor) in app.available_monitors()?.iter().enumerate() {
            if let Some(name) = monitor.name() {
                menu.append(&CheckMenuItem::with_id(
                    &app,
                    format!("monitor:{index}"),
                    name.replace('&', "&&"),
                    true,
                    prefs.monitor.as_ref() == Some(name),
                    None::<&str>,
                )?)?;
            }
        }
        menu.append(&PredefinedMenuItem::separator(&app)?)?;
        menu.append(&MenuItem::with_id(
            &app,
            "quit",
            "Quit Isle",
            true,
            None::<&str>,
        )?)?;
        Ok(menu)
    };
    window
        .popup_menu(&build().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn app_quit(app: AppHandle) {
    app.exit(0);
}

pub fn handle_menu(app: &AppHandle, id: &str) {
    if id == "quit" {
        app.exit(0);
        return;
    }
    if id.starts_with("app:") {
        match id {
            "app:open" => crate::services::wake(app, "shelf", true),
            "app:settings" => crate::services::wake(app, "settings", true),
            "app:pause" | "app:privacy" => {
                let Ok(_guard) = SETTINGS_WRITE.lock() else {
                    return;
                };
                let core = app.state::<crate::core::Core>();
                if let Ok(mut settings) = core.settings() {
                    if id == "app:pause" {
                        settings.tracking_paused = !settings.tracking_paused;
                    } else {
                        settings.presentation_mode = !settings.presentation_mode;
                    }
                    if core.save_settings(settings.clone()).is_ok() {
                        use tauri::Emitter;
                        let _ = app.emit("settings://changed", settings);
                    }
                }
            }
            "app:capture-cancel" => {
                let core = app.state::<crate::core::Core>();
                if let Ok(mut capture) = core.0.capture.lock() {
                    *capture = None;
                }
                app.state::<WindowManager>().suppress_visibility(false);
                let _ = app.emit("capture://state", false);
            }
            _ => {}
        }
        return;
    }
    let manager = app.state::<WindowManager>();
    let Ok(mut prefs) = manager.preferences() else {
        return;
    };
    if let Some(mode) = id.strip_prefix("position:") {
        prefs.position_mode = match mode {
            "taskbar" => PositionMode::Taskbar,
            "top-center" => PositionMode::TopCenter,
            "custom" => PositionMode::Custom,
            _ => return,
        };
    } else if let Some(monitor) = id.strip_prefix("monitor:") {
        if monitor == "primary" {
            prefs.monitor = None;
        } else {
            let Some(name) = monitor.parse::<usize>().ok().and_then(|i| {
                app.available_monitors()
                    .ok()
                    .and_then(|m| m.get(i).and_then(|m| m.name().cloned()))
            }) else {
                return;
            };
            prefs.monitor = Some(name);
        }
    } else {
        return;
    }
    if let Err(error) = manager.save_preferences(prefs) {
        eprintln!("Position settings could not be saved: {error}");
    }
}

use crate::core::{
    config::FeatureSettings,
    shelf::{ShelfItem, Stack},
    timer::Timer,
    Core, Note,
};
use tauri::Emitter;

#[tauri::command]
pub fn bootstrap(app: AppHandle, core: State<'_, Core>) -> Result<serde_json::Value, String> {
    let _ = core.timer_tick()?;
    let media = app
        .state::<crate::media::Media>()
        .0
        .lock()
        .map_err(|_| "Media unavailable")?
        .clone();
    Ok(
        serde_json::json!({"open_on_start":std::env::var_os("ISLE_OPEN_ON_START").is_some(),"settings":core.settings()?,"shelf":core.shelf_list()?,"stacks":core.stacks()?,"notes":core.notes()?,"timer":core.timer()?,"media":media,"vocabulary":core.vocabulary()?,"chrome":crate::platform::chrome_setup(),"downloads":core.0.downloads.lock().map_err(|_|"Downloads unavailable")?.clone(),"hotkey_errors":core.kv_get::<Vec<String>>("hotkey_errors")?.unwrap_or_default()}),
    )
}
#[tauri::command]
pub fn shelf_list(core: State<'_, Core>) -> Result<Vec<ShelfItem>, String> {
    core.shelf_list()
}
#[tauri::command]
pub fn shelf_add_files(
    app: AppHandle,
    core: State<'_, Core>,
    paths: Vec<String>,
) -> Result<Vec<ShelfItem>, String> {
    let items = core.shelf_files(paths, false)?;
    for item in &items {
        app.emit("shelf://item-added", item)
            .map_err(|e| e.to_string())?;
    }
    Ok(items)
}
#[tauri::command]
pub fn shelf_add_text(
    app: AppHandle,
    core: State<'_, Core>,
    text: String,
) -> Result<ShelfItem, String> {
    let item = core.shelf_text(text, false)?;
    app.emit("shelf://item-added", &item)
        .map_err(|e| e.to_string())?;
    Ok(item)
}
#[tauri::command]
pub fn shelf_paste(app: AppHandle, core: State<'_, Core>) -> Result<(), String> {
    let item = match crate::clipboard::read(true)? {
        crate::clipboard::Content::Text(t) => core.shelf_text(t, false)?,
        crate::clipboard::Content::Image(b) => core.shelf_image(&b, false)?,
        crate::clipboard::Content::Files(p) => {
            let items = core.shelf_files(p, false)?;
            for item in items {
                app.emit("shelf://item-added", item)
                    .map_err(|e| e.to_string())?;
            }
            return Ok(());
        }
    };
    app.emit("shelf://item-added", item)
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn shelf_remove(
    app: AppHandle,
    core: State<'_, Core>,
    id: String,
    delete_cache: bool,
) -> Result<(), String> {
    core.shelf_remove(&id, delete_cache)?;
    app.emit("shelf://changed", ()).map_err(|e| e.to_string())
}
#[tauri::command]
pub fn shelf_update(
    app: AppHandle,
    core: State<'_, Core>,
    id: String,
    name: Option<String>,
    pinned: Option<bool>,
    stack: Option<String>,
    promote: bool,
) -> Result<(), String> {
    core.shelf_update(&id, name, pinned, stack, promote)?;
    app.emit("shelf://changed", ()).map_err(|e| e.to_string())
}
#[tauri::command]
pub fn shelf_copy(
    window: WebviewWindow,
    core: State<'_, Core>,
    ids: Vec<String>,
) -> Result<(), String> {
    let owner = window.hwnd().map_err(|e| e.to_string())?.0 as usize;
    if ids.len() == 1 {
        let item = core.shelf_item(&ids[0])?;
        if let Some(text) = item.content {
            return crate::clipboard::write_text(owner, &text);
        }
        if item.kind == "image" {
            if let Some(path) = item.path {
                return crate::clipboard::write_image(owner, std::path::Path::new(&path));
            }
        }
    }
    let files = core.file_paths(&ids)?;
    crate::clipboard::write_files(owner, &files)
}
#[tauri::command]
pub async fn shelf_start_drag(
    app: AppHandle,
    window: WebviewWindow,
    core: State<'_, Core>,
    ids: Vec<String>,
) -> Result<(), String> {
    let paths = core.file_paths(&ids)?;
    drag_files(app, window, paths, |_| {}).await
}
/// Starts a native file drag out of Isle and waits for it to end. `finished`
/// runs on the UI thread with the result before `shelf://drag-finished` fires.
pub(crate) async fn drag_files(
    app: AppHandle,
    window: WebviewWindow,
    paths: Vec<std::path::PathBuf>,
    finished: impl Fn(drag::DragResult) + Send + 'static,
) -> Result<(), String> {
    let manager = app.state::<WindowManager>().inner().clone();
    manager.external_drag(true);
    let (sender, receiver) = std::sync::mpsc::channel();
    let handle = app.clone();
    let reset = manager.clone();
    if let Err(e) = app.run_on_main_thread(move || {
        let result = drag::start_drag(
            &window,
            drag::DragItem::Files(paths),
            drag::Image::Raw(include_bytes!("../icons/32x32.png").to_vec()),
            move |result, _| {
                let label = format!("{result:?}");
                finished(result);
                let _ = handle.emit("shelf://drag-finished", label);
            },
            drag::Options::default(),
        )
        .map_err(|e| e.to_string());
        reset.external_drag(false);
        let _ = sender.send(result);
    }) {
        manager.external_drag(false);
        return Err(e.to_string());
    }
    tauri::async_runtime::spawn_blocking(move || receiver.recv().map_err(|e| e.to_string())?)
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn shelf_open(core: State<'_, Core>, id: String, in_folder: bool) -> Result<(), String> {
    let item = core.shelf_item(&id)?;
    if let Some(path) = item.path {
        if !std::path::Path::new(&path).exists() {
            return Err("Original file is missing".into());
        }
        if in_folder {
            crate::platform::show_in_folder(&path)
        } else {
            crate::platform::shell_open(&path)
        }
    } else if item.kind == "link" {
        crate::platform::shell_open(item.content.as_deref().ok_or("Link is empty")?)
    } else {
        Err("Use Copy to transfer this text".into())
    }
}
#[tauri::command]
pub fn shelf_clean(core: State<'_, Core>, id: String, action: String) -> Result<String, String> {
    let item = core.shelf_item(&id)?;
    crate::core::shelf::clean_text(
        item.content
            .as_deref()
            .ok_or("Select a text or link item")?,
        &action,
    )
}
#[tauri::command]
pub fn stacks_list(core: State<'_, Core>) -> Result<Vec<Stack>, String> {
    core.stacks()
}
#[tauri::command]
pub fn stack_add(core: State<'_, Core>, name: String) -> Result<Vec<Stack>, String> {
    core.stack_add(&name)?;
    core.stacks()
}
#[tauri::command]
pub fn stack_remove(core: State<'_, Core>, id: String) -> Result<Vec<Stack>, String> {
    core.stack_remove(&id)?;
    core.stacks()
}
#[tauri::command]
pub fn notes_list(core: State<'_, Core>) -> Result<Vec<Note>, String> {
    core.notes()
}
#[tauri::command]
pub fn note_save(core: State<'_, Core>, note: Note) -> Result<(), String> {
    core.note_save(note)
}
#[tauri::command]
pub fn note_remove(core: State<'_, Core>, id: String) -> Result<(), String> {
    core.note_remove(&id)
}
#[tauri::command]
pub async fn dictionary_lookup(
    core: State<'_, Core>,
    word: String,
    refresh: bool,
) -> Result<crate::core::dictionary::DictionaryResult, String> {
    core.inner().clone().dictionary_lookup(word, refresh).await
}
#[tauri::command]
pub fn dictionary_save(
    core: State<'_, Core>,
    word: String,
    saved: bool,
) -> Result<Vec<String>, String> {
    core.dictionary_save(&word, saved)?;
    core.vocabulary()
}
#[tauri::command]
pub fn usage_summary(
    core: State<'_, Core>,
    range: String,
) -> Result<crate::usage::Summary, String> {
    if !["today", "week"].contains(&range.as_str()) {
        return Err("Choose today or week".into());
    }
    core.usage_summary(&range)
}
#[tauri::command]
pub fn usage_clear(core: State<'_, Core>) -> Result<(), String> {
    core.usage_clear()
}
#[tauri::command]
pub fn usage_snooze(core: State<'_, Core>, id: String) -> Result<(), String> {
    core.kv_set(&format!("snooze:{id}"), &(crate::core::now() + 900_000))?;
    core.kv_set(
        &format!("nudge:{}:{id}", chrono::Local::now().format("%Y-%m-%d")),
        &false,
    )
}
#[tauri::command]
pub fn timer_action(
    app: AppHandle,
    core: State<'_, Core>,
    action: String,
) -> Result<Timer, String> {
    let timer = core.timer_action(&action)?;
    app.emit("timer://changed", &timer)
        .map_err(|e| e.to_string())?;
    Ok(timer)
}
#[tauri::command]
pub async fn media_command(command: String, position: Option<f64>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        // SAFETY: Initializes and balances WinRT on this worker; an already initialized
        // apartment is left untouched if RoInitialize reports changed mode.
        let initialized = unsafe {
            windows::Win32::System::WinRT::RoInitialize(
                windows::Win32::System::WinRT::RO_INIT_MULTITHREADED,
            )
        }
        .is_ok();
        let result = crate::media::command(&command, position);
        if initialized {
            unsafe {
                windows::Win32::System::WinRT::RoUninitialize();
            }
        }
        result
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn chrome_open(core: State<'_, Core>, service: String) -> Result<(), String> {
    crate::platform::chrome_open(&core.settings()?, &service)
}
/// Serializes every read-modify-write of the feature settings so a tray toggle
/// and a panel save can't overwrite each other.
pub static SETTINGS_WRITE: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[tauri::command]
pub fn settings_set(
    app: AppHandle,
    core: State<'_, Core>,
    settings: FeatureSettings,
) -> Result<FeatureSettings, String> {
    let _guard = SETTINGS_WRITE.lock().map_err(|_| "Settings busy")?;
    apply_settings(&app, &core, settings)
}

/// Applies only the keys present in `patch` on top of the current settings.
#[tauri::command]
pub fn settings_patch(
    app: AppHandle,
    core: State<'_, Core>,
    patch: serde_json::Map<String, serde_json::Value>,
) -> Result<FeatureSettings, String> {
    let _guard = SETTINGS_WRITE.lock().map_err(|_| "Settings busy")?;
    let next = core.settings()?.merged(patch)?;
    apply_settings(&app, &core, next)
}

fn apply_settings(
    app: &AppHandle,
    core: &Core,
    settings: FeatureSettings,
) -> Result<FeatureSettings, String> {
    use tauri_plugin_autostart::ManagerExt;
    settings.validate()?;
    settings.validate_chrome_executable()?;
    let old = core.settings()?;
    if settings.hotkeys != old.hotkeys {
        let errors = crate::services::register_hotkeys(app, &settings);
        if !errors.is_empty() {
            let _ = crate::services::register_hotkeys(app, &old);
            return Err(errors.join("; "));
        }
    }
    if settings.launch_at_startup != old.launch_at_startup {
        if settings.launch_at_startup {
            app.autolaunch().enable()
        } else {
            app.autolaunch().disable()
        }
        .map_err(|e| e.to_string())?;
    }
    core.save_settings(settings.clone())?;
    app.emit("settings://changed", &settings)
        .map_err(|e| e.to_string())?;
    Ok(settings)
}
#[tauri::command]
pub fn download_action(core: State<'_, Core>, id: String, add: bool) -> Result<(), String> {
    let download = core
        .0
        .downloads
        .lock()
        .map_err(|_| "Downloads unavailable")?
        .iter()
        .find(|d| d.id == id)
        .cloned()
        .ok_or("Download no longer available")?;
    if add {
        core.shelf_files(vec![download.path], false)?;
    }
    core.0
        .downloads
        .lock()
        .map_err(|_| "Downloads unavailable")?
        .retain(|d| d.id != id);
    Ok(())
}
#[tauri::command]
pub async fn capture_start(
    app: AppHandle,
    window: WebviewWindow,
    core: State<'_, Core>,
) -> Result<(), String> {
    *core.0.capture.lock().map_err(|_| "Capture unavailable")? =
        Some((crate::clipboard::sequence(), std::time::Instant::now()));
    app.state::<WindowManager>().suppress_visibility(true);
    window.hide().map_err(|e| e.to_string())?;
    app.emit("capture://state", true)
        .map_err(|e| e.to_string())?;
    let result = tauri::async_runtime::spawn_blocking(|| {
        std::thread::sleep(std::time::Duration::from_millis(150));
        crate::platform::shell_open("ms-screenclip:")
    })
    .await
    .map_err(|e| e.to_string())?;
    if result.is_err() {
        *core.0.capture.lock().map_err(|_| "Capture unavailable")? = None;
        app.state::<WindowManager>().suppress_visibility(false);
        let _ = app.emit("capture://state", false);
    }
    result
}
