//! Which apps are using the camera, microphone or screen capture right now.
//! Windows records this per app under the CapabilityAccessManager consent
//! store (the same data behind the taskbar privacy indicator): an entry whose
//! `LastUsedTimeStop` is 0 is in use. Read only; nothing is kept.

use super::SensorUse;
use windows::{
    core::{HSTRING, PCWSTR, PWSTR},
    Win32::{
        Foundation::ERROR_SUCCESS,
        System::Registry::{
            RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER,
            KEY_READ, RRF_RT_REG_QWORD,
        },
    },
};

const STORE: &str =
    r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore";
const KINDS: [(&str, &str); 4] = [
    ("graphicsCaptureProgrammatic", "screen"),
    ("graphicsCaptureWithoutBorder", "screen"),
    ("webcam", "camera"),
    ("microphone", "microphone"),
];

struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful RegOpenKeyExW and is closed once.
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}
fn open(parent: HKEY, path: &str) -> Option<Key> {
    let mut key = HKEY::default();
    let path = HSTRING::from(path);
    // SAFETY: `path` outlives the call and `key` is a valid out pointer.
    let status = unsafe { RegOpenKeyExW(parent, &path, Some(0), KEY_READ, &mut key) };
    (status == ERROR_SUCCESS).then_some(Key(key))
}
fn subkeys(key: &Key) -> Vec<String> {
    let mut names = Vec::new();
    for index in 0..512 {
        let mut buffer = [0u16; 512];
        let mut length = buffer.len() as u32;
        // SAFETY: `buffer` holds `length` UTF-16 units and both outlive the call.
        let status = unsafe {
            RegEnumKeyExW(
                key.0,
                index,
                Some(PWSTR(buffer.as_mut_ptr())),
                &mut length,
                None,
                None,
                None,
                None,
            )
        };
        if status != ERROR_SUCCESS {
            break;
        }
        names.push(String::from_utf16_lossy(&buffer[..length as usize]));
    }
    names
}
fn qword(key: &Key, name: &str) -> Option<u64> {
    let mut value = 0u64;
    let mut size = std::mem::size_of::<u64>() as u32;
    let name = HSTRING::from(name);
    // SAFETY: `value` is an 8-byte buffer and `size` says so.
    let status = unsafe {
        RegGetValueW(
            key.0,
            PCWSTR::null(),
            &name,
            RRF_RT_REG_QWORD,
            None,
            Some(&mut value as *mut u64 as *mut _),
            Some(&mut size),
        )
    };
    (status == ERROR_SUCCESS).then_some(value)
}

/// Readable app name from a consent store key: a packaged app's family name
/// (`MSTeams_8wekyb3d8bbwe`) or an exe path with `#` for `\`.
pub fn app_name(key: &str) -> String {
    let base = if key.contains('#') {
        let file = key.rsplit('#').next().unwrap_or(key);
        file.strip_suffix(".exe")
            .or_else(|| file.strip_suffix(".EXE"))
            .unwrap_or(file)
    } else {
        let family = key.split('_').next().unwrap_or(key);
        family.rsplit('.').next().unwrap_or(family)
    };
    match base.to_lowercase().as_str() {
        "zoom" => "Zoom".into(),
        "ms-teams" | "msteams" | "teams" => "Teams".into(),
        "chrome" => "Chrome".into(),
        "msedge" | "msedgewebview2" => "Edge".into(),
        "firefox" => "Firefox".into(),
        "discord" => "Discord".into(),
        "slack" => "Slack".into(),
        "obs64" | "obs32" => "OBS".into(),
        "windowscamera" => "Camera".into(),
        "windowssoundrecorder" => "Sound Recorder".into(),
        "screensketch" | "snippingtool" => "Snipping Tool".into(),
        _ => base.into(),
    }
}
/// FILETIME (100 ns since 1601) to Unix milliseconds.
pub fn unix_ms(filetime: u64) -> i64 {
    (filetime / 10_000) as i64 - 11_644_473_600_000
}
fn entry(key: &Key, name: &str, kind: &str) -> Option<SensorUse> {
    let start = qword(key, "LastUsedTimeStart")?;
    (qword(key, "LastUsedTimeStop")? == 0 && start > 0).then(|| SensorUse {
        kind: kind.into(),
        app: app_name(name),
        since: Some(unix_ms(start)),
    })
}

pub fn in_use() -> Vec<SensorUse> {
    let mut found: Vec<SensorUse> = Vec::new();
    for (store, kind) in KINDS {
        let Some(root) = open(HKEY_CURRENT_USER, &format!(r"{STORE}\{store}")) else {
            continue;
        };
        for name in subkeys(&root) {
            let Some(child) = open(root.0, &name) else {
                continue;
            };
            let entries: Vec<SensorUse> = if name == "NonPackaged" {
                subkeys(&child)
                    .into_iter()
                    .filter_map(|app| entry(&open(child.0, &app)?, &app, kind))
                    .collect()
            } else {
                entry(&child, &name, kind).into_iter().collect()
            };
            for item in entries {
                if !found
                    .iter()
                    .any(|f| f.kind == item.kind && f.app == item.app)
                {
                    found.push(item);
                }
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn app_names_read_like_people_say_them() {
        assert_eq!(
            app_name("C:#Users#me#AppData#Roaming#Zoom#bin#Zoom.exe"),
            "Zoom"
        );
        assert_eq!(app_name("MSTeams_8wekyb3d8bbwe"), "Teams");
        assert_eq!(app_name("Microsoft.WindowsCamera_8wekyb3d8bbwe"), "Camera");
        assert_eq!(
            app_name("C:#Program Files#Google#Chrome#Application#chrome.exe"),
            "Chrome"
        );
        assert_eq!(app_name("C:#Tools#Recorder.EXE"), "Recorder");
    }
    #[test]
    fn filetime_converts_to_unix_ms() {
        assert_eq!(unix_ms(116_444_736_000_000_000), 0);
        assert_eq!(unix_ms(116_444_736_000_000_000 + 10_000_000), 1_000);
    }
}
