use crate::core::{now, Core};
use chrono::{Local, TimeZone, Timelike};
use rusqlite::params;
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashMap},
    hash::{Hash, Hasher},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};
use windows::Win32::{
    Foundation::HWND,
    System::{
        Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED},
        SystemInformation::GetTickCount,
    },
    UI::{
        Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK},
        Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO},
        WindowsAndMessaging::*,
    },
};

static CHANGED: AtomicBool = AtomicBool::new(true);
unsafe extern "system" fn foreground_event(
    _: HWINEVENTHOOK,
    _: u32,
    _: HWND,
    _: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    CHANGED.store(true, Ordering::Release);
}
#[derive(Clone, Debug, Serialize)]
pub struct AppUsage {
    pub id: String,
    pub name: String,
    pub path: String,
    pub color: String,
    pub icon: Option<String>,
    pub seconds: f64,
}
#[derive(Debug, Serialize)]
pub struct Bucket {
    pub label: String,
    pub seconds: f64,
    pub apps: BTreeMap<String, f64>,
}
#[derive(Debug, Serialize)]
pub struct Summary {
    pub total_seconds: f64,
    pub yesterday_seconds: f64,
    pub apps: Vec<AppUsage>,
    pub buckets: Vec<Bucket>,
    pub paused: bool,
    pub range: String,
}
fn day_start(date: chrono::NaiveDate) -> Result<i64, String> {
    Local
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).ok_or("Invalid date")?)
        .earliest()
        .map(|d| d.timestamp_millis())
        .ok_or("Local day unavailable".into())
}
pub fn clipped_duration(start: i64, end: i64, from: i64, to: i64) -> i64 {
    (end.min(to) - start.max(from)).max(0)
}
impl Core {
    pub fn usage_summary(&self, range: &str) -> Result<Summary, String> {
        let config = self.settings()?;
        let today = Local::now().date_naive();
        let from = if range == "week" {
            today - chrono::Duration::days(6)
        } else {
            today
        };
        let start = day_start(from)?;
        let end = day_start(today + chrono::Duration::days(1))?;
        let yesterday = day_start(today - chrono::Duration::days(1))?;
        let today_start = day_start(today)?;
        let db = self.0.db.lock().map_err(|_| "Database unavailable")?;
        let mut apps = HashMap::new();
        let mut statement = db
            .prepare("SELECT id,display_name,exe_path,color,icon_png FROM apps")
            .map_err(|e| e.to_string())?;
        for item in statement
            .query_map([], |r| {
                Ok(AppUsage {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    path: r.get(2)?,
                    color: r.get(3)?,
                    icon: r.get(4)?,
                    seconds: 0.0,
                })
            })
            .map_err(|e| e.to_string())?
        {
            let app = item.map_err(|e| e.to_string())?;
            if !config.excluded_apps.contains(&app.id) {
                apps.insert(app.id.clone(), app);
            }
        }
        let mut buckets: Vec<Bucket> = if range == "week" {
            (0..7)
                .map(|i| Bucket {
                    label: (from + chrono::Duration::days(i)).format("%a").to_string(),
                    seconds: 0.0,
                    apps: BTreeMap::new(),
                })
                .collect()
        } else {
            (0..24)
                .map(|i| Bucket {
                    label: format!("{i:02}"),
                    seconds: 0.0,
                    apps: BTreeMap::new(),
                })
                .collect()
        };
        let mut yesterday_seconds = 0.0;
        let mut query = db
            .prepare("SELECT app_id,start_ts,end_ts FROM sessions WHERE start_ts<?1 AND end_ts>?2")
            .map_err(|e| e.to_string())?;
        let rows = query
            .query_map(params![end, start.min(yesterday)], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (id, a, b) = row.map_err(|e| e.to_string())?;
            let Some(app) = apps.get_mut(&id) else {
                continue;
            };
            yesterday_seconds += clipped_duration(a, b, yesterday, today_start) as f64 / 1000.0;
            app.seconds += clipped_duration(a, b, start, end) as f64 / 1000.0;
            let mut cursor = a.max(start);
            let until = b.min(end);
            while cursor < until {
                let Some(local) = Local.timestamp_millis_opt(cursor).single() else {
                    break;
                };
                let index = if range == "week" {
                    (local.date_naive() - from).num_days() as usize
                } else {
                    local.hour() as usize
                };
                let boundary = if range == "week" {
                    day_start(local.date_naive() + chrono::Duration::days(1))?
                } else {
                    cursor + ((3600 - local.minute() * 60 - local.second()) as i64) * 1000
                        - i64::from(local.timestamp_subsec_millis())
                };
                let next = boundary.min(until).max(cursor + 1);
                if let Some(bucket) = buckets.get_mut(index) {
                    let seconds = (next - cursor) as f64 / 1000.0;
                    bucket.seconds += seconds;
                    *bucket.apps.entry(id.clone()).or_default() += seconds;
                }
                cursor = next;
            }
        }
        let mut apps = apps
            .into_values()
            .filter(|a| a.seconds > 0.0)
            .collect::<Vec<_>>();
        apps.sort_by(|a, b| b.seconds.total_cmp(&a.seconds));
        let total_seconds = apps.iter().map(|a| a.seconds).sum();
        Ok(Summary {
            total_seconds,
            yesterday_seconds,
            apps,
            buckets,
            paused: config.tracking_paused,
            range: range.into(),
        })
    }
    pub fn usage_clear(&self) -> Result<(), String> {
        let mut db = self.0.db.lock().map_err(|_| "Database unavailable")?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM sessions", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM apps", [])
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
}
fn app_info(core: &Core, window: HWND) -> Option<(String, Option<String>)> {
    let path = crate::platform::process_path(window)?;
    let string = path.to_string_lossy().to_string();
    if path
        .file_stem()
        .is_some_and(|s| s.eq_ignore_ascii_case("isle"))
    {
        return None;
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    string.to_lowercase().hash(&mut hasher);
    let id = format!("{:016x}", hasher.finish());
    let settings = core.settings().ok()?;
    if settings.excluded_apps.contains(&id) {
        return None;
    }
    let db = core.0.db.lock().ok()?;
    let count: i32 = db
        .query_row("SELECT count(*) FROM apps WHERE id=?1", [&id], |r| r.get(0))
        .ok()?;
    drop(db);
    if count == 0 {
        let name = crate::platform::product_name(&path);
        let thumbnail = core
            .0
            .cache
            .join("thumbnails")
            .join(format!("app-{id}.png"));
        let mut color = format!("hsl({} 65% 70%)", hasher.finish() % 360);
        let icon = crate::platform::shell_thumbnail(&path, 32)
            .ok()
            .and_then(|image| {
                let pixels = image.to_rgba8();
                let (mut r, mut g, mut b, mut n) = (0u64, 0u64, 0u64, 0u64);
                for p in pixels.pixels() {
                    if p[3] > 128 && p[0].max(p[1]).max(p[2]) - p[0].min(p[1]).min(p[2]) > 30 {
                        r += u64::from(p[0]);
                        g += u64::from(p[1]);
                        b += u64::from(p[2]);
                        n += 1;
                    }
                }
                if let (Some(r), Some(g), Some(b)) =
                    (r.checked_div(n), g.checked_div(n), b.checked_div(n))
                {
                    color = format!("#{r:02x}{g:02x}{b:02x}");
                }
                image.save(&thumbnail).ok()?;
                Some(thumbnail.to_string_lossy().to_string())
            });
        core.0.db.lock().ok()?.execute("INSERT OR IGNORE INTO apps(id,exe_path,display_name,color,icon_png) VALUES(?1,?2,?3,?4,?5)",params![id,string,name,color,icon]).ok()?;
    }
    let title = if settings.track_window_titles {
        // SAFETY: Optional title capture is explicit opt-in; buffer is bounded
        // and this reads only the current foreground window, storing locally.
        let mut buffer = [0u16; 1024];
        let len = unsafe { GetWindowTextW(window, &mut buffer) };
        Some(String::from_utf16_lossy(&buffer[..len.max(0) as usize]))
    } else {
        None
    };
    Some((id, title))
}
pub fn start(app: AppHandle, core: Core) {
    std::thread::spawn(move || {
        // SAFETY: This worker owns its COM apartment and out-of-context foreground
        // hook. It pumps messages without injection and unhooks before COM cleanup.
        let com = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
        let hook = unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(foreground_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            )
        };
        let mut message = MSG::default();
        let mut last_sample = Instant::now() - Duration::from_secs(2);
        let mut current: Option<(String, Option<String>)> = None;
        let mut session: Option<i64> = None;
        let mut last_time = now();
        let mut notice_check = Instant::now();
        loop {
            // SAFETY: MSG is an owned valid out-buffer; dispatched messages belong
            // only to this dedicated hook thread.
            unsafe {
                while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            let changed = CHANGED.swap(false, Ordering::AcqRel);
            if changed || last_sample.elapsed() >= Duration::from_secs(1) {
                last_sample = Instant::now();
                let time = now();
                let config = match core.settings() {
                    Ok(c) => c,
                    Err(_) => break,
                };
                let mut input = LASTINPUTINFO {
                    cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
                    ..Default::default()
                };
                // SAFETY: Reads only input-idle timestamp and foreground HWND; it
                // never records keystrokes or mouse coordinates.
                let (idle, window) = unsafe {
                    let valid = GetLastInputInfo(&mut input).as_bool();
                    (
                        if valid {
                            GetTickCount().wrapping_sub(input.dwTime)
                        } else {
                            u32::MAX
                        },
                        GetForegroundWindow(),
                    )
                };
                let inactive = config.tracking_paused
                    || crate::window::session_inactive()
                    || idle >= config.idle_timeout_seconds * 1000;
                let next = if inactive {
                    None
                } else {
                    app_info(&core, window)
                };
                let discontinuity = time - last_time > 5000 || time < last_time;
                if let Ok(db) = core.0.db.lock() {
                    if let Some(id) = session {
                        let end = if discontinuity {
                            last_time
                        } else if idle >= config.idle_timeout_seconds * 1000 {
                            time - i64::from(idle - config.idle_timeout_seconds * 1000)
                        } else {
                            time
                        };
                        if db
                            .execute(
                                "UPDATE sessions SET end_ts=MAX(start_ts,?1) WHERE id=?2",
                                params![end, id],
                            )
                            .ok()
                            == Some(0)
                        {
                            session = None;
                        }
                    }
                    if next != current || discontinuity || session.is_none() {
                        session = None;
                        if let Some((id, title)) = &next {
                            let _=db.execute("INSERT INTO sessions(app_id,start_ts,end_ts,title) VALUES(?1,?2,?2,?3)",params![id,time,title]);
                            session = Some(db.last_insert_rowid());
                        }
                    }
                }
                current = next;
                last_time = time;
                if notice_check.elapsed() >= Duration::from_secs(5) {
                    notice_check = Instant::now();
                    let _ = app.emit("usage://tick", ());
                    if let Ok(summary) = core.usage_summary("today") {
                        for item in summary.apps {
                            if let Some(limit) = config.app_limits.get(&item.id) {
                                if *limit > 0 && item.seconds >= f64::from(*limit) * 60.0 {
                                    let key = format!(
                                        "nudge:{}:{}",
                                        Local::now().format("%Y-%m-%d"),
                                        item.id
                                    );
                                    let snooze = core
                                        .kv_get::<i64>(&format!("snooze:{}", item.id))
                                        .ok()
                                        .flatten()
                                        .unwrap_or(0);
                                    if snooze <= now()
                                        && core.kv_get::<bool>(&key).ok().flatten() != Some(true)
                                    {
                                        let _ = core.kv_set(&key, &true);
                                        let _=app.emit("usage://limit",serde_json::json!({"id":item.id,"name":item.name,"minutes":limit}));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(50));
            if app.get_webview_window("main").is_none() {
                break;
            }
        }
        // SAFETY: Balances resources allocated on this same worker thread.
        unsafe {
            if !hook.0.is_null() {
                let _ = UnhookWinEvent(hook);
            }
            if com {
                CoUninitialize();
            }
        }
    });
}
use tauri::Manager;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aggregate_clips_sessions_at_day_boundaries() {
        assert_eq!(clipped_duration(900, 2100, 1000, 2000), 1000);
        assert_eq!(clipped_duration(0, 900, 1000, 2000), 0);
    }
}
