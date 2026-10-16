//! Live activities: things happening right now that the pill can morph to
//! show. Battery and charging, browser downloads still in progress, and which
//! apps are using the camera, microphone or screen capture. Everything is read
//! locally from Windows; nothing is stored or sent anywhere.

pub mod audio;
mod sensors;

use crate::core::Core;
use serde::Serialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};

#[derive(Clone, Debug, Serialize, Default, PartialEq)]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
    pub plugged_in: bool,
    pub saver: bool,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ActiveDownload {
    pub name: String,
    pub bytes: u64,
    pub bytes_per_second: u64,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct SensorUse {
    /// `camera`, `microphone` or `screen`.
    pub kind: String,
    pub app: String,
    /// Unix milliseconds when the app started using it, if Windows recorded it.
    pub since: Option<i64>,
}
#[derive(Clone, Debug, Serialize, Default, PartialEq)]
pub struct LiveActivity {
    pub battery: Option<Battery>,
    pub downloads: Vec<ActiveDownload>,
    pub sensors: Vec<SensorUse>,
}
#[derive(Clone, Default)]
pub struct Activity(pub Arc<Mutex<LiveActivity>>);

const PARTIAL: [&str; 4] = [".crdownload", ".part", ".partial", ".download"];

/// Display name of an in-progress download, or `None` if `file` is not one.
/// Chrome's early `Unconfirmed 123.crdownload` placeholder reads as "Download".
pub fn partial_name(file: &str) -> Option<String> {
    let lower = file.to_lowercase();
    let suffix = PARTIAL.iter().find(|s| lower.ends_with(*s))?;
    let base = &file[..file.len() - suffix.len()];
    if base.is_empty() {
        return None;
    }
    Some(if base.starts_with("Unconfirmed ") {
        "Download".into()
    } else {
        base.into()
    })
}

/// Tracks partial files between scans so each one gets a transfer rate.
#[derive(Default)]
pub struct DownloadTracker {
    seen: HashMap<PathBuf, (u64, Instant, u64)>,
}
impl DownloadTracker {
    /// `files` are (path, size, seconds since last write). A file that has not
    /// grown for a minute is treated as stalled and left out.
    pub fn update(&mut self, files: Vec<(PathBuf, u64, u64)>, now: Instant) -> Vec<ActiveDownload> {
        let mut next = HashMap::new();
        let mut active = Vec::new();
        for (path, bytes, idle) in files {
            let Some(name) = path
                .file_name()
                .and_then(|n| partial_name(&n.to_string_lossy()))
            else {
                continue;
            };
            let rate = match self.seen.get(&path) {
                Some((before, at, rate)) => {
                    let elapsed = now.duration_since(*at).as_secs_f64();
                    if elapsed >= 0.5 {
                        // Light smoothing so the speed doesn't jitter every scan.
                        let instant = bytes.saturating_sub(*before) as f64 / elapsed;
                        (instant * 0.6 + *rate as f64 * 0.4) as u64
                    } else {
                        *rate
                    }
                }
                None => 0,
            };
            next.insert(path, (bytes, now, rate));
            if idle < 60 {
                active.push(ActiveDownload {
                    name,
                    bytes,
                    bytes_per_second: rate,
                });
            }
        }
        self.seen = next;
        active.sort_by(|a, b| a.name.cmp(&b.name));
        active
    }
}

fn scan_partials(folder: &Path) -> Vec<(PathBuf, u64, u64)> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if !PARTIAL.iter().any(|s| name.ends_with(s)) {
                return None;
            }
            let meta = entry.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            let idle = meta
                .modified()
                .ok()
                .and_then(|m| m.elapsed().ok())
                .map_or(0, |d| d.as_secs());
            Some((entry.path(), meta.len(), idle))
        })
        .take(32)
        .collect()
}

fn battery() -> Option<Battery> {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut status = SYSTEM_POWER_STATUS::default();
    // SAFETY: `status` is a valid, writable SYSTEM_POWER_STATUS for the call.
    unsafe { GetSystemPowerStatus(&mut status) }.ok()?;
    battery_from(
        status.ACLineStatus,
        status.BatteryFlag,
        status.BatteryLifePercent,
        status.SystemStatusFlag,
    )
}
/// Maps SYSTEM_POWER_STATUS fields; desktops without a battery give `None`.
pub fn battery_from(ac: u8, flag: u8, percent: u8, system: u8) -> Option<Battery> {
    if flag == 128 || flag == 255 || percent > 100 {
        return None;
    }
    Some(Battery {
        percent,
        charging: flag & 8 != 0,
        plugged_in: ac == 1,
        saver: system & 1 != 0,
    })
}

#[tauri::command]
pub fn activity_get(activity: tauri::State<'_, Activity>) -> Result<LiveActivity, String> {
    activity
        .0
        .lock()
        .map(|a| a.clone())
        .map_err(|_| "Live activity unavailable".into())
}
/// The pill turns the loudness meter on only while music plays and motion is allowed.
#[tauri::command]
pub fn audio_meter(meter: tauri::State<'_, audio::Meter>, enabled: bool) {
    meter.0.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

pub fn start(app: AppHandle, core: Core, activity: Activity) {
    std::thread::spawn(move || {
        let mut tracker = DownloadTracker::default();
        let mut tick: u32 = 0;
        let mut current = LiveActivity::default();
        loop {
            let settings = core.settings().ok();
            let downloads = match &settings {
                Some(s) if s.watch_downloads => tracker.update(
                    scan_partials(Path::new(&s.downloads_folder)),
                    Instant::now(),
                ),
                _ => Vec::new(),
            };
            let mut next = LiveActivity {
                downloads,
                // Battery and sensors change slowly; read them every other second.
                battery: if tick.is_multiple_of(2) {
                    battery()
                } else {
                    current.battery.clone()
                },
                sensors: if tick.is_multiple_of(2) {
                    sensors::in_use()
                } else {
                    current.sensors.clone()
                },
            };
            if settings.is_some_and(|s| s.presentation_mode) {
                next.sensors.clear();
                next.downloads.clear();
            }
            if next != current {
                current = next.clone();
                if let Ok(mut shared) = activity.0.lock() {
                    *shared = next.clone();
                }
                let _ = app.emit("activity://changed", &next);
            }
            tick = tick.wrapping_add(1);
            std::thread::sleep(Duration::from_secs(1));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_names_strip_the_browser_suffix() {
        assert_eq!(
            partial_name("setup.exe.crdownload").as_deref(),
            Some("setup.exe")
        );
        assert_eq!(partial_name("Movie.MKV.PART").as_deref(), Some("Movie.MKV"));
        assert_eq!(
            partial_name("Unconfirmed 81234.crdownload").as_deref(),
            Some("Download")
        );
        assert_eq!(partial_name("notes.txt"), None);
        assert_eq!(partial_name(".part"), None);
    }
    #[test]
    fn tracker_reports_speed_and_drops_stalled_files() {
        let mut tracker = DownloadTracker::default();
        let t0 = Instant::now();
        let file = PathBuf::from("C:/Downloads/a.zip.crdownload");
        let first = tracker.update(vec![(file.clone(), 1_000, 0)], t0);
        assert_eq!(first[0].bytes_per_second, 0);
        let second = tracker.update(vec![(file.clone(), 3_000, 0)], t0 + Duration::from_secs(1));
        assert_eq!(second[0].name, "a.zip");
        assert_eq!(second[0].bytes_per_second, 1_200);
        assert!(tracker
            .update(vec![(file, 3_000, 120)], t0 + Duration::from_secs(2))
            .is_empty());
        assert!(tracker
            .update(vec![], t0 + Duration::from_secs(3))
            .is_empty());
    }
    #[test]
    fn battery_status_maps_windows_flags() {
        assert_eq!(battery_from(1, 128, 255, 0), None);
        assert_eq!(battery_from(0, 255, 50, 0), None);
        let b = battery_from(1, 8 | 1, 54, 1).unwrap();
        assert!(b.charging && b.plugged_in && b.saver);
        assert_eq!(b.percent, 54);
        assert!(!battery_from(0, 2, 12, 0).unwrap().charging);
    }
}
