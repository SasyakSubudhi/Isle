pub mod geometry;

use geometry::Rect;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::HWND,
        System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER},
        UI::{
            Accessibility::{
                CUIAutomation, IUIAutomation, TreeScope_Descendants, UIA_ButtonControlTypeId,
                UIA_ListItemControlTypeId, UIA_TextControlTypeId,
            },
            Shell::{SHAppBarMessage, ABM_GETSTATE, ABM_GETTASKBARPOS, ABS_AUTOHIDE, APPBARDATA},
            WindowsAndMessaging::{FindWindowExW, FindWindowW, GetWindowRect, IsWindowVisible},
        },
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Taskbar {
    pub hwnd: usize,
    pub rect: Rect,
    pub auto_hide: bool,
    pub visible: bool,
}

pub fn find(monitor: Rect, primary: bool) -> Option<Taskbar> {
    // SAFETY: Class names are static UTF-16 strings. Returned Explorer HWNDs are
    // borrowed only for this snapshot; every rectangle query handles destruction.
    unsafe {
        let mut candidates = Vec::new();
        if let Ok(hwnd) = FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null()) {
            candidates.push(hwnd);
        }
        let mut after = None;
        while let Ok(hwnd) =
            FindWindowExW(None, after, w!("Shell_SecondaryTrayWnd"), PCWSTR::null())
        {
            candidates.push(hwnd);
            after = Some(hwnd);
        }
        let mut data = APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            ..Default::default()
        };
        let auto_hide = SHAppBarMessage(ABM_GETSTATE, &mut data) & ABS_AUTOHIDE as usize != 0;
        for hwnd in candidates {
            let mut native = Default::default();
            if GetWindowRect(hwnd, &mut native).is_err() {
                continue;
            }
            let mut rect = Rect::from(native);
            let visible = IsWindowVisible(hwnd).as_bool()
                && rect.top >= monitor.top
                && rect.bottom <= monitor.bottom + 2
                && rect.height() > 4;
            if primary && SHAppBarMessage(ABM_GETTASKBARPOS, &mut data) != 0 {
                let appbar = Rect::from(data.rc);
                if appbar.intersects(monitor) {
                    rect = appbar;
                }
            }
            if rect.intersects(monitor) {
                return Some(Taskbar {
                    hwnd: hwnd.0 as usize,
                    rect,
                    auto_hide,
                    visible,
                });
            }
        }
        None
    }
}

/// Called only on the dedicated COM-initialized placement thread. Never move
/// UIA interfaces to the cursor polling thread or the Tauri UI thread.
pub fn occupied(taskbar: Taskbar) -> Result<Vec<Rect>, String> {
    if taskbar.auto_hide && !taskbar.visible {
        return Err("Auto-hide taskbar is currently hidden".into());
    }
    // SAFETY: COM is initialized by the caller on this same thread; all interface
    // objects and returned arrays stay in this apartment. Invalid/stale elements
    // are skipped and an empty result causes the explicit placement fallback.
    unsafe {
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| e.to_string())?;
        let root = automation
            .ElementFromHandle(HWND(taskbar.hwnd as *mut _))
            .map_err(|e| e.to_string())?;
        let condition = automation
            .CreateTrueCondition()
            .map_err(|e| e.to_string())?;
        let elements = root
            .FindAll(TreeScope_Descendants, &condition)
            .map_err(|e| e.to_string())?;
        let mut rects = Vec::new();
        for index in 0..elements.Length().map_err(|e| e.to_string())?.min(2000) {
            let Ok(element) = elements.GetElement(index) else {
                continue;
            };
            if element
                .CurrentIsOffscreen()
                .map(|v| v.as_bool())
                .unwrap_or(true)
            {
                continue;
            }
            let Ok(kind) = element.CurrentControlType() else {
                continue;
            };
            if ![
                UIA_ButtonControlTypeId,
                UIA_ListItemControlTypeId,
                UIA_TextControlTypeId,
            ]
            .contains(&kind)
            {
                continue;
            }
            if let Ok(native) = element.CurrentBoundingRectangle() {
                let rect = Rect::from(native);
                if rect.width() > 0 && rect.height() > 0 && rect.intersects(taskbar.rect) {
                    rects.push(rect);
                }
            }
        }
        if rects.is_empty() {
            Err("Explorer returned no usable taskbar controls".into())
        } else {
            Ok(rects)
        }
    }
}
