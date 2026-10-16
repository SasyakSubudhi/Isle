//! One OLE file-drop adapter for the outer HWND and WebView child HWNDs.
//! The system owns a COM reference until RevokeDragDrop/window destruction.
use crate::{core::Core, window::WindowManager};
use std::{
    collections::HashSet,
    io::Write,
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use windows::{
    core::implement,
    Win32::{
        Foundation::{HWND, LPARAM, POINTL},
        System::{
            Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL},
            Ole::{
                IDropTarget, IDropTarget_Impl, RegisterDragDrop, ReleaseStgMedium, RevokeDragDrop,
                CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_NONE,
            },
            SystemServices::MODIFIERKEYS_FLAGS,
        },
        UI::{
            Shell::{DragQueryFileW, HDROP},
            WindowsAndMessaging::{EnumChildWindows, GetClassNameW, IsWindow},
        },
    },
};

fn diagnostic(message: impl std::fmt::Display) {
    static LOCK: Mutex<()> = Mutex::new(());
    let Ok(_guard) = LOCK.lock() else {
        return;
    };
    let Some(local) = std::env::var_os("LOCALAPPDATA") else {
        return;
    };
    let path = std::path::PathBuf::from(local).join("Isle/drop-diagnostics.log");
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > 65_536) {
        let _ = std::fs::write(&path, b"");
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        // Local, bounded diagnostics contain event/format results, never paths or content.
        let _ = writeln!(file, "{} {message}", crate::core::now());
    }
}

fn paths(data: windows_core::Ref<'_, IDataObject>) -> windows::core::Result<Vec<String>> {
    let format = FORMATETC {
        cfFormat: CF_HDROP.0,
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
        ..Default::default()
    };
    // SAFETY: IDataObject is borrowed for this synchronous OLE callback. The
    // returned storage medium is released exactly once after bounded path reads.
    unsafe {
        let object = data
            .as_ref()
            .ok_or_else(|| windows::core::Error::from(windows::Win32::Foundation::E_POINTER))?;
        let mut medium = object.GetData(&format)?;
        let handle = HDROP(medium.u.hGlobal.0);
        let count = DragQueryFileW(handle, u32::MAX, None).min(256);
        let mut files = Vec::new();
        for index in 0..count {
            let length = DragQueryFileW(handle, index, None);
            if length > 0 && length < 32768 {
                let mut buffer = vec![0; length as usize + 1];
                DragQueryFileW(handle, index, Some(&mut buffer));
                files.push(String::from_utf16_lossy(&buffer[..length as usize]));
            }
        }
        ReleaseStgMedium(&mut medium);
        Ok(files)
    }
}

#[implement(IDropTarget)]
struct FileDrop {
    app: AppHandle,
    core: Core,
    manager: WindowManager,
    valid: Mutex<bool>,
    hover_state: Mutex<Option<bool>>,
    address: usize,
}
impl FileDrop {
    fn hover(&self, point: &POINTL, effect: *mut DROPEFFECT) -> windows::core::Result<()> {
        let valid = self.valid.lock().is_ok_and(|valid| *valid)
            && !self.manager.dragging_out()
            && self.manager.accepts_drop(point.x, point.y);
        if let Ok(mut previous) = self.hover_state.lock() {
            if *previous != Some(valid) {
                diagnostic(format!("hover hwnd={} accepted={valid}", self.address));
                *previous = Some(valid);
            }
        }
        // SAFETY: OLE provides an effect out-pointer for the duration of each callback.
        unsafe {
            if let Some(effect) = effect.as_mut() {
                *effect = if valid {
                    DROPEFFECT_COPY
                } else {
                    DROPEFFECT_NONE
                };
            }
        }
        let _ = self.app.emit("shelf://native-drop-hover", valid);
        Ok(())
    }
}
#[allow(non_snake_case)]
impl IDropTarget_Impl for FileDrop_Impl {
    fn DragEnter(
        &self,
        data: windows_core::Ref<'_, IDataObject>,
        _: MODIFIERKEYS_FLAGS,
        point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        let files = paths(data);
        diagnostic(match &files {
            Ok(files) => format!("enter hwnd={} files={}", self.address, files.len()),
            Err(error) => format!(
                "enter hwnd={} CF_HDROP error={:08x}",
                self.address,
                error.code().0
            ),
        });
        let count = files.as_ref().map_or(0, Vec::len);
        if let Ok(mut valid) = self.valid.lock() {
            *valid = count > 0;
        }
        // Lets the pill say how many files it is about to dock.
        let _ = self.app.emit("dock://drag-count", count);
        self.hover(point, effect)
    }
    fn DragOver(
        &self,
        _: MODIFIERKEYS_FLAGS,
        point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        self.hover(point, effect)
    }
    fn DragLeave(&self) -> windows::core::Result<()> {
        diagnostic(format!("leave hwnd={}", self.address));
        if let Ok(mut valid) = self.valid.lock() {
            *valid = false;
        }
        let _ = self.app.emit("shelf://native-drop-hover", false);
        Ok(())
    }
    fn Drop(
        &self,
        data: windows_core::Ref<'_, IDataObject>,
        _: MODIFIERKEYS_FLAGS,
        point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        let files = paths(data).unwrap_or_default();
        let accept = !files.is_empty()
            && !self.manager.dragging_out()
            && self.manager.accepts_drop(point.x, point.y);
        diagnostic(format!(
            "drop hwnd={} files={} accepted={accept}",
            self.address,
            files.len()
        ));
        // SAFETY: OLE owns the valid effect out-pointer throughout this callback.
        unsafe {
            if let Some(effect) = effect.as_mut() {
                *effect = if accept {
                    DROPEFFECT_COPY
                } else {
                    DROPEFFECT_NONE
                };
            }
        }
        let _ = self.DragLeave();
        if accept {
            let core = self.core.clone();
            let app = self.app.clone();
            std::thread::spawn(move || match core.shelf_files(files, false) {
                Ok(items) => {
                    diagnostic(format!("shelf added={}", items.len()));
                    for item in &items {
                        let _ = app.emit("shelf://item-added", item);
                    }
                    if let Err(error) = crate::dock::dock_items(&app, &core, &items) {
                        let _ = app.emit("app://error", error);
                    }
                }
                Err(error) => {
                    diagnostic("shelf add failed; error shown in app");
                    let _ = app.emit("app://error", error);
                }
            });
        }
        Ok(())
    }
}
fn register(app: &AppHandle, core: &Core, hwnd: HWND) -> Result<(), String> {
    let target: IDropTarget = FileDrop {
        manager: app.state::<WindowManager>().inner().clone(),
        app: app.clone(),
        core: core.clone(),
        valid: Mutex::new(false),
        hover_state: Mutex::new(None),
        address: hwnd.0 as usize,
    }
    .into();
    // SAFETY: Called on the OLE-initialized UI thread for our own window subtree.
    // Replace Wry/Tao targets so child hit testing cannot bypass this adapter.
    unsafe {
        let _ = RevokeDragDrop(hwnd);
        RegisterDragDrop(hwnd, &target).map_err(|e| format!("Outer file-drop target: {e}"))?;
    }
    Ok(())
}

fn install_subtree(
    app: &AppHandle,
    core: &Core,
    address: usize,
    installed: &Mutex<HashSet<usize>>,
) -> Result<(), String> {
    let root = HWND(address as *mut _);
    let mut windows = vec![root];
    unsafe extern "system" fn collect(window: HWND, context: LPARAM) -> windows_core::BOOL {
        // SAFETY: EnumChildWindows calls synchronously; context points to the
        // owned Vec for the entire enumeration and no pointer escapes.
        unsafe {
            (&mut *(context.0 as *mut Vec<HWND>)).push(window);
        }
        true.into()
    }
    // SAFETY: Enumerates only descendants of our own live HWND; callback storage lives here.
    unsafe {
        let _ = EnumChildWindows(
            Some(root),
            Some(collect),
            LPARAM((&mut windows as *mut Vec<HWND>) as isize),
        );
    }
    let mut installed = installed
        .lock()
        .map_err(|_| "Drop registration unavailable")?;
    for window in windows {
        let id = window.0 as usize;
        if installed.contains(&id) {
            continue;
        }
        let result = register(app, core, window);
        let mut class = [0u16; 128];
        // SAFETY: Bounded output buffer and enumerated HWND; failure is an empty class.
        let length = unsafe { GetClassNameW(window, &mut class) }.max(0) as usize;
        let class = String::from_utf16_lossy(&class[..length]);
        match result {
            Ok(()) => {
                installed.insert(id);
                diagnostic(format!("registered hwnd={id} class={class}"));
            }
            Err(error) => {
                diagnostic(format!(
                    "registration failed hwnd={id} class={class} {error}"
                ));
                if id == address {
                    return Err(error);
                }
            }
        }
    }
    Ok(())
}
pub fn install(app: AppHandle, window: &WebviewWindow, core: Core) -> Result<(), String> {
    let address = window.hwnd().map_err(|e| e.to_string())?.0 as usize;
    let installed = Arc::new(Mutex::new(HashSet::new()));
    install_subtree(&app, &core, address, &installed)?;
    let cleanup = installed.clone();
    window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
            if let Ok(targets) = cleanup.lock() {
                for address in targets.iter() {
                    // SAFETY: Balances our own subtree registrations on the UI thread.
                    unsafe {
                        let _ = RevokeDragDrop(HWND(*address as *mut _));
                    }
                }
            }
        }
    });
    // WebView can create replacement child HWNDs after startup. Registration
    // always runs on the UI thread; unchanged children are never re-registered.
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        // SAFETY: IsWindow only queries the borrowed root HWND and tolerates destruction.
        if !unsafe { IsWindow(Some(HWND(address as *mut _))) }.as_bool() {
            break;
        }
        let handle = app.clone();
        let core = core.clone();
        let installed = installed.clone();
        let _ = app.run_on_main_thread(move || {
            let _ = install_subtree(&handle, &core, address, &installed);
        });
    });
    Ok(())
}
