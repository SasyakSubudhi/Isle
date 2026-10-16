use super::Signals;
use crate::taskbar::geometry::Rect;
use std::{
    sync::{atomic::Ordering, Arc, OnceLock},
    time::Instant,
};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
    UI::{
        Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK},
        Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON},
        Shell::{
            DefSubclassProc, RemoveWindowSubclass, SHQueryUserNotificationState, SetWindowSubclass,
            QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
        },
        WindowsAndMessaging::*,
    },
};

static SIGNALS: OnceLock<Arc<Signals>> = OnceLock::new();
/// Broadcast by Explorer when it (re)creates the taskbar, e.g. after a crash
/// or restart. Placement is recomputed against the new taskbar.
static TASKBAR_CREATED: OnceLock<u32> = OnceLock::new();
static CLOCK: OnceLock<Instant> = OnceLock::new();
static SESSION_INACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn session_inactive() -> bool {
    SESSION_INACTIVE.load(Ordering::Acquire)
}
const SUBCLASS_ID: usize = 0x49534c45;

pub fn millis() -> u64 {
    CLOCK.get_or_init(Instant::now).elapsed().as_millis() as u64
}
pub fn hwnd(address: usize) -> HWND {
    HWND(address as *mut _)
}

pub fn class_name(window: HWND) -> String {
    let mut name = [0u16; 128];
    // SAFETY: The bounded mutable UTF-16 buffer is valid for the entire call.
    // Explorer may destroy the HWND; zero length is treated as an unknown class.
    let length = unsafe { GetClassNameW(window, &mut name) };
    String::from_utf16_lossy(&name[..length.max(0) as usize])
}

pub fn rect(window: HWND) -> Option<Rect> {
    let mut result = Default::default();
    // SAFETY: result is a valid out-parameter and stale HWNDs return an error.
    unsafe { GetWindowRect(window, &mut result) }
        .ok()
        .map(|_| result.into())
}

pub fn input() -> Option<(POINT, bool)> {
    let mut point = POINT::default();
    // SAFETY: Cursor out-parameter is valid; only mouse-button state is queried,
    // never typed keys or application text.
    unsafe {
        GetCursorPos(&mut point).ok()?;
        Some((point, GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0))
    }
}

pub fn focus_style(address: usize, focusable: bool) -> Result<(), String> {
    // SAFETY: address belongs to our live Tauri window. Only extended style bits
    // documented for activation/taskbar behavior are modified; other bits remain.
    unsafe {
        let window = hwnd(address);
        let mut style = GetWindowLongPtrW(window, GWL_EXSTYLE) as u32;
        style = (style | WS_EX_TOOLWINDOW.0) & !WS_EX_APPWINDOW.0;
        if focusable {
            style &= !WS_EX_NOACTIVATE.0;
        } else {
            style |= WS_EX_NOACTIVATE.0;
        }
        SetWindowLongPtrW(window, GWL_EXSTYLE, style as isize);
        let actual = GetWindowLongPtrW(window, GWL_EXSTYLE) as u32;
        let relevant = WS_EX_TOOLWINDOW.0 | WS_EX_APPWINDOW.0 | WS_EX_NOACTIVATE.0;
        if actual & relevant != style & relevant {
            return Err("Windows did not apply the island activation styles".into());
        }
        SetWindowPos(
            window,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn foreground() -> usize {
    // SAFETY: Returns a borrowed HWND which is only compared or queried.
    unsafe { GetForegroundWindow().0 as usize }
}

pub fn fullscreen(address: usize, monitor: Rect) -> bool {
    // SAFETY: The shell function takes no pointers and returns a value/result.
    if let Ok(state) = unsafe { SHQueryUserNotificationState() } {
        if [
            QUNS_BUSY,
            QUNS_PRESENTATION_MODE,
            QUNS_RUNNING_D3D_FULL_SCREEN,
        ]
        .contains(&state)
        {
            return true;
        }
    }
    let active = foreground();
    if active == 0 || active == address {
        return false;
    }
    let window = hwnd(active);
    if [
        "Progman",
        "WorkerW",
        "Shell_TrayWnd",
        "Shell_SecondaryTrayWnd",
    ]
    .contains(&class_name(window).as_str())
    {
        return false;
    }
    rect(window).is_some_and(|r| {
        r.left <= monitor.left
            && r.top <= monitor.top
            && r.right >= monitor.right
            && r.bottom >= monitor.bottom
    })
}

pub fn topmost(address: usize) {
    // SAFETY: This is our live HWND; flags preserve geometry and never activate.
    if let Err(error) = unsafe {
        SetWindowPos(
            hwnd(address),
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    } {
        eprintln!("Isle z-order refresh failed: {error}");
    }
}

unsafe extern "system" fn subclass(
    window: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    if let Some(signals) = SIGNALS.get() {
        if message == WM_CLIPBOARDUPDATE {
            crate::clipboard::on_update();
        }
        if message == WM_WTSSESSION_CHANGE {
            if wp.0 == WTS_SESSION_LOCK as usize {
                SESSION_INACTIVE.store(true, Ordering::Release);
            }
            if wp.0 == WTS_SESSION_UNLOCK as usize {
                SESSION_INACTIVE.store(false, Ordering::Release);
                signals.invalidate();
            }
        }
        if message == WM_POWERBROADCAST {
            if wp.0 == PBT_APMSUSPEND as usize {
                SESSION_INACTIVE.store(true, Ordering::Release);
            }
            if [
                PBT_APMRESUMEAUTOMATIC as usize,
                PBT_APMRESUMESUSPEND as usize,
            ]
            .contains(&wp.0)
            {
                SESSION_INACTIVE.store(false, Ordering::Release);
                signals.invalidate();
            }
        }
        if [WM_DISPLAYCHANGE, WM_SETTINGCHANGE, WM_DPICHANGED].contains(&message)
            || TASKBAR_CREATED.get() == Some(&message)
        {
            signals.invalidate();
        }
        if message == WM_EXITSIZEMOVE {
            signals.drag_finished.store(true, Ordering::Release);
        }
        if message == WM_NCDESTROY {
            signals.quit.store(true, Ordering::Release);
            // SAFETY: Removes this exact subclass before the owned HWND dies.
            unsafe {
                let _ = RemoveWindowSubclass(window, Some(subclass), SUBCLASS_ID);
            }
        }
    }
    // SAFETY: Every message is forwarded with the original parameters; the
    // subclass neither consumes DPI messages nor changes another window's proc.
    unsafe { DefSubclassProc(window, message, wp, lp) }
}

unsafe extern "system" fn win_event(
    _: HWINEVENTHOOK,
    event: u32,
    window: HWND,
    _: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    let Some(signals) = SIGNALS.get() else {
        return;
    };
    if event == EVENT_SYSTEM_FOREGROUND {
        signals.foreground_changed.store(true, Ordering::Release);
        signals.invalidate();
        return;
    }
    // SAFETY: A possibly stale event HWND is only queried. Null/unknown roots are
    // ignored. No window belonging to Explorer is modified or subclassed.
    let root = unsafe { GetAncestor(window, GA_ROOT) };
    if ["Shell_TrayWnd", "Shell_SecondaryTrayWnd"].contains(&class_name(root).as_str()) {
        signals.invalidate();
    }
}

pub fn install(address: usize, signals: Arc<Signals>) -> Result<(), String> {
    SIGNALS
        .set(signals.clone())
        .map_err(|_| "Window manager already initialized")?;
    // SAFETY: Called during Tauri setup on the owning window thread; static
    // callback and identifier remain valid until WM_NCDESTROY removes them.
    // SAFETY: Registers (or looks up) a named message with a static string.
    let created = unsafe { RegisterWindowMessageW(windows::core::w!("TaskbarCreated")) };
    if created != 0 {
        let _ = TASKBAR_CREATED.set(created);
    }
    if !unsafe { SetWindowSubclass(hwnd(address), Some(subclass), SUBCLASS_ID, 0) }.as_bool() {
        return Err("Could not attach display change listener".into());
    }
    // SAFETY: Registration targets our own live HWND; the subclass handles only
    // documented clipboard and session notifications, never injected app input.
    unsafe {
        windows::Win32::System::DataExchange::AddClipboardFormatListener(hwnd(address))
            .map_err(|e| e.to_string())?;
        windows::Win32::System::RemoteDesktop::WTSRegisterSessionNotification(
            hwnd(address),
            windows::Win32::System::RemoteDesktop::NOTIFY_FOR_THIS_SESSION,
        )
        .map_err(|e| e.to_string())?;
    }
    std::thread::Builder::new()
        .name("isle-window-events".into())
        .spawn(move || {
            // SAFETY: Out-of-context hooks observe documented shell events without
            // injection. Callbacks are static; handles stay on this pumping thread
            // and are unhooked before it exits. MSG is a valid owned buffer.
            unsafe {
                let foreground = SetWinEventHook(
                    EVENT_SYSTEM_FOREGROUND,
                    EVENT_SYSTEM_FOREGROUND,
                    None,
                    Some(win_event),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT,
                );
                let taskbar = SetWinEventHook(
                    EVENT_OBJECT_CREATE,
                    EVENT_OBJECT_LOCATIONCHANGE,
                    None,
                    Some(win_event),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT,
                );
                if foreground.0.is_null() || taskbar.0.is_null() {
                    eprintln!("Isle event hook unavailable; periodic window checks remain active");
                }
                let mut message = MSG::default();
                while !signals.quit.load(Ordering::Acquire) {
                    while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                        let _ = TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                if !foreground.0.is_null() {
                    let _ = UnhookWinEvent(foreground);
                }
                if !taskbar.0.is_null() {
                    let _ = UnhookWinEvent(taskbar);
                }
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}
