use std::{
    path::{Path, PathBuf},
    process::Command,
};
use windows::{
    core::{w, HSTRING},
    Win32::{
        Foundation::{CloseHandle, HWND, LPARAM, SIZE},
        Graphics::Gdi::*,
        System::{
            Com::CoTaskMemFree,
            Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
        UI::{
            Shell::{
                FOLDERID_Downloads, IShellItemImageFactory, SHCreateItemFromParsingName,
                SHGetKnownFolderPath, ShellExecuteW, KF_FLAG_DEFAULT, SIIGBF_BIGGERSIZEOK,
            },
            WindowsAndMessaging::{
                EnumChildWindows, GetWindowThreadProcessId, IsWindowVisible, SW_SHOWNORMAL,
            },
        },
    },
};

pub fn downloads_folder() -> PathBuf {
    // SAFETY: Known-folder API allocates a UTF-16 string; it is converted before
    // its COM allocation is freed. Failure falls back to the user's home folder.
    unsafe {
        if let Ok(path) = SHGetKnownFolderPath(&FOLDERID_Downloads, KF_FLAG_DEFAULT, None) {
            let result = path.to_string().ok().map(PathBuf::from);
            CoTaskMemFree(Some(path.0.cast()));
            if let Some(path) = result {
                return path;
            }
        }
    }
    PathBuf::from(std::env::var_os("USERPROFILE").unwrap_or_default()).join("Downloads")
}
pub fn shell_open(target: &str) -> Result<(), String> {
    // SAFETY: All UTF-16 arguments own their storage until ShellExecute returns;
    // this opens a user-selected file/folder or the explicit snipping URI.
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            &HSTRING::from(target),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        Err("Windows could not open this item".into())
    } else {
        Ok(())
    }
}
pub fn show_in_folder(path: &str) -> Result<(), String> {
    Command::new("explorer.exe")
        .arg(format!("/select,{path}"))
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
pub fn shell_thumbnail(path: &Path, size: i32) -> Result<image::DynamicImage, String> {
    // SAFETY: Called on COM-initialized threads. Shell interfaces remain local;
    // bitmap/DC handles are released on all paths, and buffers match queried size.
    unsafe {
        let factory: IShellItemImageFactory = SHCreateItemFromParsingName(
            &HSTRING::from(path.as_os_str().to_string_lossy().as_ref()),
            None,
        )
        .map_err(|e| e.to_string())?;
        let bitmap = factory
            .GetImage(SIZE { cx: size, cy: size }, SIIGBF_BIGGERSIZEOK)
            .map_err(|e| e.to_string())?;
        let result = (|| {
            let mut info = BITMAP::default();
            if GetObjectW(
                HGDIOBJ(bitmap.0),
                std::mem::size_of::<BITMAP>() as i32,
                Some((&mut info as *mut BITMAP).cast()),
            ) == 0
            {
                return Err("Could not read shell bitmap".into());
            }
            let width = info.bmWidth;
            let height = info.bmHeight.abs();
            if width <= 0 || height <= 0 || width > 1024 || height > 1024 {
                return Err("Shell thumbnail dimensions are invalid".into());
            }
            let mut bytes = vec![0u8; (width * height * 4) as usize];
            let mut bmi = BITMAPINFO::default();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = width;
            bmi.bmiHeader.biHeight = -height;
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            let dc = GetDC(None);
            let lines = GetDIBits(
                dc,
                bitmap,
                0,
                height as u32,
                Some(bytes.as_mut_ptr().cast()),
                &mut bmi,
                DIB_RGB_COLORS,
            );
            ReleaseDC(None, dc);
            if lines == 0 {
                return Err("Shell thumbnail pixels unavailable".into());
            }
            let opaque = bytes.as_chunks::<4>().0.iter().all(|p| p[3] == 0);
            for pixel in bytes.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
                if opaque {
                    pixel[3] = 255;
                }
            }
            image::RgbaImage::from_raw(width as u32, height as u32, bytes)
                .map(image::DynamicImage::ImageRgba8)
                .ok_or("Shell thumbnail could not be decoded".into())
        })();
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        result
    }
}
struct ChildProcess {
    parent: u32,
    child: Option<u32>,
}
unsafe extern "system" fn child_process(window: HWND, param: LPARAM) -> windows::core::BOOL {
    // SAFETY: EnumChildWindows is synchronous; param references its live stack
    // state. Only visible child process identifiers are read, never their text.
    unsafe {
        let state = &mut *(param.0 as *mut ChildProcess);
        let mut pid = 0;
        GetWindowThreadProcessId(window, Some(&mut pid));
        if pid != state.parent && pid != 0 && IsWindowVisible(window).as_bool() {
            state.child = Some(pid);
            return false.into();
        }
        true.into()
    }
}
pub fn process_path(window: HWND) -> Option<PathBuf> {
    // SAFETY: Query-only process handles are closed before returning; output is
    // bounded by a 32K UTF-16 buffer. UWP frame hosts resolve visible child PID.
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(window, Some(&mut pid));
        fn query(pid: u32) -> Option<PathBuf> {
            // SAFETY: Only QUERY_LIMITED_INFORMATION is requested and the owned
            // handle is always closed, including on a failed path query.
            unsafe {
                let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
                let mut buffer = vec![0u16; 32768];
                let mut len = buffer.len() as u32;
                let result = QueryFullProcessImageNameW(
                    handle,
                    PROCESS_NAME_WIN32,
                    windows::core::PWSTR(buffer.as_mut_ptr()),
                    &mut len,
                )
                .ok();
                let _ = CloseHandle(handle);
                result.map(|_| PathBuf::from(String::from_utf16_lossy(&buffer[..len as usize])))
            }
        }
        let path = query(pid)?;
        if path.file_stem().is_some_and(|s| {
            s.to_string_lossy()
                .eq_ignore_ascii_case("ApplicationFrameHost")
        }) {
            let mut state = ChildProcess {
                parent: pid,
                child: None,
            };
            let _ = EnumChildWindows(
                Some(window),
                Some(child_process),
                LPARAM((&mut state as *mut ChildProcess) as isize),
            );
            if let Some(pid) = state.child {
                if let Some(path) = query(pid) {
                    return Some(path);
                }
            }
        }
        Some(path)
    }
}
pub fn product_name(path: &Path) -> String {
    use windows::Win32::Storage::FileSystem::*;
    // SAFETY: Version queries point into a stable byte buffer whose lifetime
    // encloses all VerQueryValue calls. Lengths are checked before dereferencing.
    unsafe {
        let name = HSTRING::from(path.to_string_lossy().as_ref());
        let size = GetFileVersionInfoSizeW(&name, None);
        if size > 0 && size < 4_000_000 {
            let mut data = vec![0u8; size as usize];
            if GetFileVersionInfoW(&name, None, size, data.as_mut_ptr().cast()).is_ok() {
                let mut ptr = std::ptr::null_mut();
                let mut len = 0;
                if VerQueryValueW(
                    data.as_ptr().cast(),
                    w!("\\VarFileInfo\\Translation"),
                    &mut ptr,
                    &mut len,
                )
                .as_bool()
                    && len >= 4
                {
                    let language = std::ptr::read_unaligned(ptr.cast::<u16>());
                    let page = std::ptr::read_unaligned(ptr.cast::<u16>().add(1));
                    let key = HSTRING::from(format!(
                        "\\StringFileInfo\\{language:04x}{page:04x}\\ProductName"
                    ));
                    if VerQueryValueW(data.as_ptr().cast(), &key, &mut ptr, &mut len).as_bool()
                        && len > 1
                        && len < 1024
                    {
                        let text = String::from_utf16_lossy(std::slice::from_raw_parts(
                            ptr.cast::<u16>(),
                            len as usize - 1,
                        ));
                        if !text.trim().is_empty() {
                            return text;
                        }
                    }
                }
            }
        }
    }
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown app".into())
}

#[derive(serde::Serialize)]
pub struct ChromeProfile {
    pub directory: String,
    pub name: String,
    pub account: String,
}
#[derive(serde::Serialize)]
pub struct ChromeSetup {
    pub executable: String,
    pub profiles: Vec<ChromeProfile>,
}
pub fn chrome_setup() -> ChromeSetup {
    let candidates = [
        std::env::var_os("PROGRAMFILES").map(PathBuf::from),
        std::env::var_os("PROGRAMFILES(X86)").map(PathBuf::from),
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
    ];
    let executable = candidates
        .into_iter()
        .flatten()
        .map(|p| p.join("Google/Chrome/Application/chrome.exe"))
        .find(|p| p.is_file())
        .map(|p| p.to_string_lossy().into())
        .unwrap_or_default();
    let local = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join("Google/Chrome/User Data/Local State");
    let data = std::fs::read(local)
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
    let profiles = data
        .as_ref()
        .and_then(|d| d.pointer("/profile/info_cache"))
        .and_then(|v| v.as_object())
        .map(|p| {
            p.iter()
                .map(|(key, value)| ChromeProfile {
                    directory: key.clone(),
                    name: value["name"].as_str().unwrap_or(key).into(),
                    account: value["user_name"].as_str().unwrap_or("").into(),
                })
                .collect()
        })
        .unwrap_or_default();
    ChromeSetup {
        executable,
        profiles,
    }
}
pub fn chrome_open(
    settings: &crate::core::config::FeatureSettings,
    service: &str,
) -> Result<(), String> {
    let setup = chrome_setup();
    let executable = if settings.chrome_executable.is_empty() {
        setup.executable
    } else {
        settings.chrome_executable.clone()
    };
    if executable.is_empty() || !Path::new(&executable).is_file() {
        return Err("Choose your Chrome executable in Settings".into());
    }
    if settings.chrome_profile.is_empty()
        || !setup
            .profiles
            .iter()
            .any(|p| p.directory == settings.chrome_profile)
    {
        return Err("Choose your signed-in Chrome profile in Settings".into());
    }
    let url = match service {
        "gmail" => &settings.gmail_url,
        "outlook" => &settings.outlook_url,
        "whatsapp" => &settings.whatsapp_url,
        _ => return Err("Unknown shortcut".into()),
    };
    Command::new(executable)
        .arg(format!("--profile-directory={}", settings.chrome_profile))
        .arg(url)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
