//! Context-aware pill. Once a second, reads which executable owns the
//! foreground window and tells the pill whether you're coding, in a meeting,
//! reading or writing, and since when. Only the executable name is used:
//! never the window title or anything inside the window.
pub mod rules;

use crate::core::{now, Core};
use rules::{Active, Tracker};
use serde::Serialize;
use std::{sync::Mutex, time::Duration};
use tauri::{AppHandle, Emitter, Manager, State};
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

#[derive(Default)]
pub struct ContextState(Mutex<Tracker>);

#[derive(Clone, Debug, Serialize)]
pub struct ContextInfo {
    pub kind: &'static str,
    pub app: String,
    pub since: i64,
}

impl From<Active> for ContextInfo {
    fn from(active: Active) -> Self {
        Self {
            kind: active.kind.as_str(),
            app: active.app,
            since: active.since,
        }
    }
}

/// The foreground executable's file stem, and whether it is Isle itself.
fn foreground_app() -> (Option<String>, bool) {
    // SAFETY: Reads only the foreground window handle; process_path opens a
    // query-only handle to read the executable path and closes it.
    let window = unsafe { GetForegroundWindow() };
    let stem = crate::platform::process_path(window)
        .and_then(|path| path.file_stem().map(|s| s.to_string_lossy().to_string()));
    let own = stem
        .as_deref()
        .is_some_and(|s| s.eq_ignore_ascii_case("isle"));
    (stem, own)
}

pub fn start(app: AppHandle, core: Core) {
    app.manage(ContextState::default());
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        if app.get_webview_window("main").is_none() {
            break;
        }
        let Ok(settings) = core.settings() else {
            continue;
        };
        let state = app.state::<ContextState>();
        let Ok(mut tracker) = state.0.lock() else {
            continue;
        };
        let change = if !settings.context_pill {
            tracker.reset().then_some(None)
        } else if crate::window::session_inactive() {
            tracker.step(None, false, now())
        } else {
            let (exe, own) = foreground_app();
            tracker.step(exe.as_deref(), own, now())
        };
        drop(tracker);
        if let Some(next) = change {
            let _ = app.emit("context://changed", next.map(ContextInfo::from));
        }
    });
}

#[tauri::command]
pub fn context_current(state: State<'_, ContextState>) -> Result<Option<ContextInfo>, String> {
    Ok(state
        .0
        .lock()
        .map_err(|_| "Context unavailable")?
        .current()
        .map(ContextInfo::from))
}
