mod native;
pub use native::session_inactive;

use crate::{
    settings::{self, PositionMode, WindowPreferences},
    taskbar::{
        self,
        geometry::{largest_gap, Rect},
    },
};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

#[derive(Default)]
pub struct Signals {
    quit: AtomicBool,
    invalidated_at: AtomicU64,
    foreground_changed: AtomicBool,
    dragging: AtomicBool,
    drag_finished: AtomicBool,
}
impl Signals {
    fn invalidate(&self) {
        self.invalidated_at
            .store(native::millis().max(1), Ordering::Release);
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Slot {
    pub window_x: i32,
    pub window_y: i32,
    pub anchor_x: f64,
    pub anchor_y: f64,
    pub scale: f64,
    pub monitor: String,
    pub monitor_rect: Rect,
    pub source: String,
    pub reason: Option<String>,
    pub taskbar: Option<Rect>,
    pub taskbar_hidden: bool,
    pub gap: Option<(i32, i32)>,
    pub pill_width: f64,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct HitRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

struct Inner {
    slot: Mutex<Option<Slot>>,
    hit: Mutex<Option<HitRegion>>,
    preferences: Mutex<WindowPreferences>,
    settings_path: PathBuf,
    signals: Arc<Signals>,
    focusable: AtomicBool,
    force_hidden: AtomicBool,
    external_drag: AtomicBool,
}

#[derive(Clone)]
pub struct WindowManager(Arc<Inner>);

impl WindowManager {
    pub fn accepts_drop(&self, x: i32, y: i32) -> bool {
        let Some(slot) = self.slot().ok().flatten() else {
            return false;
        };
        let Some(hit) = self.0.hit.lock().ok().and_then(|h| *h) else {
            return false;
        };
        let local_x = f64::from(x - slot.window_x) / slot.scale;
        let local_y = f64::from(y - slot.window_y) / slot.scale;
        local_x >= hit.x
            && local_x < hit.x + hit.width
            && local_y >= hit.y
            && local_y < hit.y + hit.height
    }
    pub fn suppress_visibility(&self, hidden: bool) {
        self.0.force_hidden.store(hidden, Ordering::Release);
    }
    pub fn external_drag(&self, active: bool) {
        self.0.external_drag.store(active, Ordering::Release);
    }
    /// True while Isle itself is dragging files out, so its own drop target can refuse them.
    pub fn dragging_out(&self) -> bool {
        self.0.external_drag.load(Ordering::Acquire)
    }
    pub fn slot(&self) -> Result<Option<Slot>, String> {
        Ok(self
            .0
            .slot
            .lock()
            .map_err(|_| "Window state unavailable")?
            .clone())
    }
    pub fn preferences(&self) -> Result<WindowPreferences, String> {
        Ok(self
            .0
            .preferences
            .lock()
            .map_err(|_| "Settings state unavailable")?
            .clone())
    }
    pub fn save_preferences(&self, prefs: WindowPreferences) -> Result<(), String> {
        let mut current = self
            .0
            .preferences
            .lock()
            .map_err(|_| "Settings state unavailable")?;
        settings::save(&self.0.settings_path, &prefs)?;
        *current = prefs;
        self.0.signals.invalidate();
        Ok(())
    }
    pub fn hit_region(&self, region: HitRegion) -> Result<(), String> {
        if ![region.x, region.y, region.width, region.height]
            .iter()
            .all(|n| n.is_finite())
            || region.width <= 0.0
            || region.height <= 0.0
            || region.x < 0.0
            || region.y < 0.0
            || region.x + region.width > 562.0
            || region.y + region.height > 362.0
        {
            return Err("Invalid island hit region".into());
        }
        *self.0.hit.lock().map_err(|_| "Hit region unavailable")? = Some(region);
        Ok(())
    }
    pub fn focusable(&self, window: &WebviewWindow, enabled: bool) -> Result<(), String> {
        window.set_focusable(enabled).map_err(|e| e.to_string())?;
        native::focus_style(
            window.hwnd().map_err(|e| e.to_string())?.0 as usize,
            enabled,
        )?;
        self.0.focusable.store(enabled, Ordering::Release);
        if enabled {
            window.set_focus().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn begin_drag(&self, window: &WebviewWindow) -> Result<(), String> {
        let slot = self.slot()?.ok_or("Placement is not ready")?;
        let mut prefs = self.preferences()?;
        prefs.position_mode = PositionMode::Custom;
        prefs.custom_x =
            (slot.window_x - slot.monitor_rect.left) as f64 / slot.scale + slot.anchor_x;
        prefs.custom_y =
            (slot.window_y - slot.monitor_rect.top) as f64 / slot.scale + slot.anchor_y;
        self.save_preferences(prefs)?;
        self.0.signals.dragging.store(true, Ordering::Release);
        if let Err(e) = window.start_dragging() {
            self.0.signals.dragging.store(false, Ordering::Release);
            return Err(e.to_string());
        }
        Ok(())
    }
}

fn monitor_data(
    app: &AppHandle,
    prefs: &WindowPreferences,
) -> Result<(Rect, f64, String, bool), String> {
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let primary = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("No primary monitor available")?;
    let selected = prefs
        .monitor
        .as_ref()
        .and_then(|name| monitors.iter().find(|m| m.name() == Some(name)))
        .unwrap_or(&primary);
    let pos = selected.position();
    let size = selected.size();
    Ok((
        Rect {
            left: pos.x,
            top: pos.y,
            right: pos.x + size.width as i32,
            bottom: pos.y + size.height as i32,
        },
        selected.scale_factor(),
        selected
            .name()
            .cloned()
            .unwrap_or_else(|| "Primary monitor".into()),
        selected.position() == primary.position(),
    ))
}

fn calculate_slot(
    monitor: Rect,
    scale: f64,
    name: String,
    prefs: &WindowPreferences,
    bar: Option<taskbar::Taskbar>,
    occupied: Result<Vec<Rect>, String>,
) -> Slot {
    let mut reason = None;
    let mut gap = None;
    let center = monitor.left + monitor.width() / 2;
    let top_y = monitor.top + (36.0 * scale).round() as i32;
    let (anchor_x, anchor_y, source) = match prefs.position_mode {
        PositionMode::TopCenter => (center, top_y, "top-center"),
        PositionMode::Custom => (
            monitor.left + (prefs.custom_x * scale).round() as i32,
            monitor.top + (prefs.custom_y * scale).round() as i32,
            "custom",
        ),
        PositionMode::Taskbar => {
            if let Some(bar) = bar {
                if bar.rect.height() > bar.rect.width()
                    || bar.rect.top < monitor.top + monitor.height() / 2
                {
                    reason = Some("This taskbar orientation uses the top-center fallback".into());
                    (center, top_y, "top-center")
                } else if (bar.rect.height() as f64) < 36.0 * scale {
                    reason = Some(
                        "Taskbar is too short for the pill at this DPI; using top-center".into(),
                    );
                    (center, top_y, "top-center")
                } else {
                    match occupied {
                        Ok(rects) => {
                            gap = largest_gap(
                                bar.rect,
                                &rects,
                                ((prefs.pill_width + 32.0) * scale).ceil() as i32,
                            );
                            if let Some((a, b)) = gap {
                                ((a + b) / 2, (bar.rect.top + bar.rect.bottom) / 2, "taskbar")
                            } else {
                                reason=Some("No taskbar gap is wide enough; choose a position from the pill's right-click menu".into());
                                (center, top_y, "top-center")
                            }
                        }
                        Err(error) => {
                            reason = Some(format!("Taskbar detection unavailable: {error}"));
                            (center, top_y, "top-center")
                        }
                    }
                }
            } else {
                reason = Some("Explorer taskbar unavailable; using top-center".into());
                (center, top_y, "top-center")
            }
        }
    };
    let half_width = ((prefs.pill_width * scale / 2.0).ceil() as i32).min(monitor.width() / 2);
    let ax = anchor_x.clamp(monitor.left + half_width, monitor.right - half_width);
    let ay = anchor_y.clamp(
        monitor.top + (18.0 * scale).ceil() as i32,
        monitor.bottom - (18.0 * scale).ceil() as i32,
    );
    let canvas_w = (560.0 * scale).round() as i32;
    let canvas_h = (360.0 * scale).round() as i32;
    let wx = (ax - canvas_w / 2).clamp(monitor.left, (monitor.right - canvas_w).max(monitor.left));
    let wy = (ay + (30.0 * scale).round() as i32 - canvas_h)
        .clamp(monitor.top, (monitor.bottom - canvas_h).max(monitor.top));
    Slot {
        window_x: wx,
        window_y: wy,
        anchor_x: (ax - wx) as f64 / scale,
        anchor_y: (ay - wy) as f64 / scale,
        scale,
        monitor: name,
        monitor_rect: monitor,
        source: source.into(),
        reason,
        taskbar: bar.map(|b| b.rect),
        taskbar_hidden: prefs.position_mode == PositionMode::Taskbar
            && bar.is_some_and(|b| b.auto_hide && !b.visible),
        gap,
        pill_width: prefs.pill_width,
    }
}

pub fn start(app: AppHandle) -> Result<WindowManager, String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Missing main window")?;
    let address = window.hwnd().map_err(|e| e.to_string())?.0 as usize;
    let settings_path = settings::path()?;
    let preferences = settings::load(&settings_path).unwrap_or_else(|error| {
        eprintln!(
            "Isle settings could not load; defaults are active and the file is preserved: {error}"
        );
        WindowPreferences::default()
    });
    let signals = Arc::new(Signals::default());
    let manager = WindowManager(Arc::new(Inner {
        slot: Mutex::new(None),
        hit: Mutex::new(None),
        preferences: Mutex::new(preferences),
        settings_path,
        signals: signals.clone(),
        focusable: AtomicBool::new(false),
        force_hidden: AtomicBool::new(false),
        external_drag: AtomicBool::new(false),
    }));
    native::install(address, signals.clone())?;
    manager.focusable(&window, false)?;
    window
        .set_ignore_cursor_events(true)
        .map_err(|e| e.to_string())?;

    let placement = manager.clone();
    let placement_app = app.clone();
    let placement_window = window.clone();
    std::thread::Builder::new()
        .name("isle-placement".into())
        .spawn(move || {
            // SAFETY: COM initialization and uninitialization occur on this dedicated
            // worker. UIA objects are local to taskbar::occupied and never escape it.
            let com = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
            let mut previous_bar = None;
            let mut last_detection = 0;
            let mut last_attempt = 0;
            while !placement.0.signals.quit.load(Ordering::Acquire) {
                if placement
                    .0
                    .signals
                    .drag_finished
                    .swap(false, Ordering::AcqRel)
                    && placement.0.signals.dragging.swap(false, Ordering::AcqRel)
                {
                    if let (Ok(Some(slot)), Ok(pos), Ok(mut prefs)) = (
                        placement.slot(),
                        placement_window.outer_position(),
                        placement.preferences(),
                    ) {
                        prefs.custom_x =
                            (pos.x - slot.monitor_rect.left) as f64 / slot.scale + slot.anchor_x;
                        prefs.custom_y =
                            (pos.y - slot.monitor_rect.top) as f64 / slot.scale + slot.anchor_y;
                        if let Err(e) = placement.save_preferences(prefs) {
                            eprintln!("Custom position could not be saved: {e}");
                        }
                    }
                }
                if placement.0.signals.dragging.load(Ordering::Acquire) {
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                let Ok(prefs) = placement.preferences() else {
                    break;
                };
                if let Ok((monitor, scale, name, primary)) = monitor_data(&placement_app, &prefs) {
                    let bar = taskbar::find(monitor, primary);
                    let dirty = placement.0.signals.invalidated_at.load(Ordering::Acquire);
                    let now = native::millis();
                    let previous = placement.slot().ok().flatten();
                    let geometry_changed = previous.as_ref().is_none_or(|s| {
                        s.monitor_rect != monitor || s.scale != scale || s.monitor != name
                    });
                    // Poll only geometry once a second; rerun UIA after a 500ms event
                    // debounce, on geometry changes, or every 30s as a recovery check.
                    if previous.is_none()
                        || geometry_changed
                        || bar != previous_bar
                        || (dirty > last_detection && now.saturating_sub(dirty) >= 500)
                        || now.saturating_sub(last_attempt) > 30_000
                    {
                        last_attempt = now;
                        let occupied = if com {
                            bar.map(taskbar::occupied)
                                .unwrap_or_else(|| Err("Taskbar missing".into()))
                        } else {
                            Err("UI Automation COM initialization failed".into())
                        };
                        let detection_failed = occupied.is_err();
                        let mut slot = calculate_slot(monitor, scale, name, &prefs, bar, occupied);
                        // Cache a good slot only while the taskbar geometry and chosen
                        // monitor remain identical; stale slots must not cross displays.
                        if slot.source == "top-center"
                            && detection_failed
                            && prefs.position_mode == PositionMode::Taskbar
                            && bar == previous_bar
                            && !geometry_changed
                        {
                            if let Some(old) = previous.as_ref().filter(|s| {
                                s.source == "taskbar" && s.pill_width == prefs.pill_width
                            }) {
                                let reason = slot.reason;
                                slot = old.clone();
                                slot.reason =
                                    reason.map(|r| format!("Using last verified taskbar gap: {r}"));
                            }
                        }
                        if previous.as_ref() != Some(&slot) {
                            let result = (|| -> Result<(), String> {
                                if previous.is_none() || geometry_changed {
                                    placement_window
                                        .set_size(PhysicalSize::new(
                                            (560.0 * scale).round() as u32,
                                            (360.0 * scale).round() as u32,
                                        ))
                                        .map_err(|e| e.to_string())?;
                                }
                                // Initial native positioning must precede publication:
                                // the pointer thread may show the canvas as soon as a slot exists.
                                if previous.is_none() || geometry_changed {
                                    placement_window
                                        .set_position(PhysicalPosition::new(
                                            slot.window_x,
                                            slot.window_y,
                                        ))
                                        .map_err(|e| e.to_string())?;
                                }
                                *placement
                                    .0
                                    .slot
                                    .lock()
                                    .map_err(|_| "Placement state unavailable")? =
                                    Some(slot.clone());
                                placement_app
                                    .emit("display://changed", &slot)
                                    .map_err(|e| e.to_string())?;
                                if let Some(old) = previous.as_ref().filter(|old| {
                                    old.monitor == slot.monitor
                                        && old.scale == slot.scale
                                        && old.source == "taskbar"
                                        && slot.source == "taskbar"
                                }) {
                                    // Translate the fixed canvas; never resize it to animate a pill.
                                    for step in 1..=12 {
                                        let t = step as f64 / 12.0;
                                        let eased = 1.0 - (1.0 - t).powi(3);
                                        let x = old.window_x
                                            + ((slot.window_x - old.window_x) as f64 * eased)
                                                .round()
                                                as i32;
                                        let y = old.window_y
                                            + ((slot.window_y - old.window_y) as f64 * eased)
                                                .round()
                                                as i32;
                                        placement_window
                                            .set_position(PhysicalPosition::new(x, y))
                                            .map_err(|e| e.to_string())?;
                                        std::thread::sleep(Duration::from_millis(16));
                                    }
                                } else {
                                    placement_window
                                        .set_position(PhysicalPosition::new(
                                            slot.window_x,
                                            slot.window_y,
                                        ))
                                        .map_err(|e| e.to_string())?;
                                }
                                eprintln!(
                                    "Isle placement: {} on {} at ({}, {}), scale={}{}",
                                    slot.source,
                                    slot.monitor,
                                    slot.window_x,
                                    slot.window_y,
                                    slot.scale,
                                    slot.reason
                                        .as_ref()
                                        .map(|r| format!("; {r}"))
                                        .unwrap_or_default()
                                );
                                Ok(())
                            })();
                            if let Err(e) = result {
                                eprintln!("Isle placement failed: {e}");
                            }
                        }
                        previous_bar = bar;
                        last_detection = dirty;
                    }
                }
                std::thread::sleep(Duration::from_millis(500));
            }
            if com {
                // SAFETY: Balances successful CoInitializeEx on this same thread,
                // after every apartment-bound UIA interface has been dropped.
                unsafe {
                    CoUninitialize();
                }
            }
        })
        .map_err(|e| e.to_string())?;

    let polling = manager.clone();
    std::thread::Builder::new()
        .name("isle-pointer".into())
        .spawn(move || {
            let mut interactive = false;
            let mut hovering = false;
            let mut hidden = true;
            let mut last_topmost = 0;
            while !polling.0.signals.quit.load(Ordering::Acquire) {
                let Some(slot) = polling.slot().ok().flatten() else {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                };
                let should_hide = polling.0.force_hidden.load(Ordering::Acquire)
                    || slot.taskbar_hidden
                    || native::fullscreen(address, slot.monitor_rect);
                if should_hide != hidden {
                    let foreground_before = native::foreground();
                    let result = if should_hide {
                        window.hide()
                    } else {
                        window.show()
                    };
                    if result.is_ok() {
                        hidden = should_hide;
                        if !hidden {
                            eprintln!(
                                "Isle shown without changing foreground: {}",
                                foreground_before == native::foreground()
                            );
                        }
                        let _ = app.emit("island://visibility", !hidden);
                    }
                }
                if hidden {
                    std::thread::sleep(Duration::from_millis(250));
                    continue;
                }
                let now = native::millis();
                if now.saturating_sub(last_topmost) >= 1000
                    || polling
                        .0
                        .signals
                        .foreground_changed
                        .swap(false, Ordering::AcqRel)
                {
                    native::topmost(address);
                    last_topmost = now;
                }
                let region = polling
                    .0
                    .hit
                    .lock()
                    .ok()
                    .and_then(|r| *r)
                    .unwrap_or(HitRegion {
                        x: slot.anchor_x - slot.pill_width / 2.0,
                        y: slot.anchor_y - 18.0,
                        width: slot.pill_width,
                        height: 36.0,
                    });
                let mut near = false;
                if let (Some((cursor, _)), Some(frame)) =
                    (native::input(), native::rect(native::hwnd(address)))
                {
                    let pill = Rect {
                        left: frame.left + (region.x * slot.scale).floor() as i32,
                        top: frame.top + (region.y * slot.scale).floor() as i32,
                        right: frame.left + ((region.x + region.width) * slot.scale).ceil() as i32,
                        bottom: frame.top + ((region.y + region.height) * slot.scale).ceil() as i32,
                    };
                    let over = pill.contains(cursor.x, cursor.y, 0);
                    near = pill.contains(cursor.x, cursor.y, (180.0 * slot.scale) as i32);
                    let capture = over
                        || polling.0.signals.dragging.load(Ordering::Acquire)
                        || polling.0.external_drag.load(Ordering::Acquire);
                    if capture != interactive && window.set_ignore_cursor_events(!capture).is_ok() {
                        interactive = capture;
                    }
                    if over != hovering {
                        hovering = over;
                        let _ = app.emit("island://pointer", over);
                    }
                }
                std::thread::sleep(Duration::from_millis(if near { 16 } else { 100 }));
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(manager)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn monitor() -> Rect {
        Rect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        }
    }
    #[test]
    fn hidden_auto_hide_taskbar_suppresses_the_island() {
        let bar = taskbar::Taskbar {
            hwnd: 1,
            rect: Rect {
                left: 0,
                top: 1032,
                right: 1920,
                bottom: 1080,
            },
            auto_hide: true,
            visible: false,
        };
        let slot = calculate_slot(
            monitor(),
            1.0,
            "test".into(),
            &WindowPreferences::default(),
            Some(bar),
            Err("Taskbar hidden".into()),
        );
        assert!(slot.taskbar_hidden);
        let top_center = WindowPreferences {
            position_mode: PositionMode::TopCenter,
            ..Default::default()
        };
        let slot = calculate_slot(
            monitor(),
            1.0,
            "test".into(),
            &top_center,
            Some(bar),
            Err("Taskbar hidden".into()),
        );
        assert!(!slot.taskbar_hidden);
    }
    #[test]
    fn failed_detection_has_visible_on_screen_fallback() {
        for scale in [1.0, 1.25, 1.5] {
            let slot = calculate_slot(
                monitor(),
                scale,
                "test".into(),
                &WindowPreferences::default(),
                None,
                Err("UIA failed".into()),
            );
            assert_eq!(slot.source, "top-center");
            assert!(slot.window_y >= 0);
            assert_eq!(slot.anchor_y, 36.0);
            assert!(slot.reason.is_some());
        }
    }
    #[test]
    fn taskbar_anchor_sits_in_gap_at_each_scale() {
        for scale in [1.0, 1.25, 1.5] {
            let bar = taskbar::Taskbar {
                hwnd: 1,
                rect: Rect {
                    left: 0,
                    top: 1080 - (48.0_f64 * scale).round() as i32,
                    right: 1920,
                    bottom: 1080,
                },
                auto_hide: false,
                visible: true,
            };
            let occupied = vec![
                Rect {
                    left: 0,
                    right: 900,
                    ..bar.rect
                },
                Rect {
                    left: 1700,
                    right: 1920,
                    ..bar.rect
                },
            ];
            let slot = calculate_slot(
                monitor(),
                scale,
                "test".into(),
                &WindowPreferences::default(),
                Some(bar),
                Ok(occupied),
            );
            assert_eq!(slot.source, "taskbar");
            assert!((slot.window_x as f64 + slot.anchor_x * scale - 1300.0).abs() < 0.01);
            let center_y = (bar.rect.top + bar.rect.bottom) as f64 / 2.0;
            assert!((slot.window_y as f64 + slot.anchor_y * scale - center_y).abs() <= 0.5);
        }
    }
}
