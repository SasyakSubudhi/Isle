use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
    time::Duration,
};
use windows::{
    core::w,
    Win32::{
        Foundation::{HANDLE, HGLOBAL},
        System::{DataExchange::*, Memory::*},
        UI::Shell::{DragQueryFileW, HDROP},
    },
};

static UPDATED: AtomicBool = AtomicBool::new(true);
static OWN_SEQUENCE: AtomicU32 = AtomicU32::new(0);
pub fn on_update() {
    UPDATED.store(true, Ordering::Release);
}
pub fn sequence() -> u32 {
    // SAFETY: The sequence API has no pointers and does not open/read content.
    unsafe { GetClipboardSequenceNumber() }
}
struct Clipboard;
impl Drop for Clipboard {
    fn drop(&mut self) {
        // SAFETY: This guard is constructed only after OpenClipboard succeeds on
        // this same thread; it always balances that call.
        unsafe {
            let _ = CloseClipboard();
        }
    }
}
fn open() -> Result<Clipboard, String> {
    for _ in 0..5 {
        // SAFETY: Clipboard ownership is scoped to this thread/RAII guard. Null HWND
        // is sufficient for reads; writes use our application HWND below.
        if unsafe { OpenClipboard(None) }.is_ok() {
            return Ok(Clipboard);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Err("Clipboard is busy. Try again.".into())
}
fn open_write(owner: usize) -> Result<Clipboard, String> {
    for _ in 0..5 {
        // SAFETY: owner is the live main window. SetClipboardData requires a real
        // owner after EmptyClipboard; OpenClipboard(NULL) is never used for writes.
        if unsafe { OpenClipboard(Some(windows::Win32::Foundation::HWND(owner as *mut _))) }.is_ok()
        {
            return Ok(Clipboard);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Err("Clipboard is busy. Try again.".into())
}
fn bytes(format: u32) -> Result<Vec<u8>, String> {
    // SAFETY: Clipboard is open on this thread. Global memory is locked only
    // while copying the bounded bytes; the clipboard retains ownership.
    unsafe {
        let handle = GetClipboardData(format).map_err(|e| e.to_string())?;
        let global = HGLOBAL(handle.0);
        let size = GlobalSize(global);
        if size == 0 || size > 32_000_000 {
            return Err("Clipboard format cannot be safely preserved".into());
        }
        let ptr = GlobalLock(global);
        if ptr.is_null() {
            return Err("Clipboard memory unavailable".into());
        }
        let result = std::slice::from_raw_parts(ptr.cast::<u8>(), size).to_vec();
        let _ = GlobalUnlock(global);
        Ok(result)
    }
}
fn put(format: u32, data: &[u8]) -> Result<(), String> {
    // SAFETY: Owned movable global memory is populated while locked. Successful
    // SetClipboardData transfers ownership to Windows; errors free it locally.
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, data.len()).map_err(|e| e.to_string())?;
        let pointer = GlobalLock(memory);
        if pointer.is_null() {
            let _ = windows::Win32::Foundation::GlobalFree(Some(memory));
            return Err("Could not allocate clipboard memory".into());
        }
        std::ptr::copy_nonoverlapping(data.as_ptr(), pointer.cast::<u8>(), data.len());
        let _ = GlobalUnlock(memory);
        if let Err(e) = SetClipboardData(format, Some(HANDLE(memory.0))) {
            let _ = windows::Win32::Foundation::GlobalFree(Some(memory));
            return Err(e.to_string());
        }
        Ok(())
    }
}
fn format(name: &windows::core::PCWSTR) -> u32 {
    // SAFETY: Callers pass static, nul-terminated format names.
    unsafe { RegisterClipboardFormatW(*name) }
}
fn private() -> bool {
    // SAFETY: Only availability and a bounded DWORD format value are inspected.
    unsafe {
        let excluded = format(&w!("ExcludeClipboardContentFromMonitorProcessing"));
        if IsClipboardFormatAvailable(excluded).is_ok() {
            return true;
        }
        let include = format(&w!("CanIncludeInClipboardHistory"));
        IsClipboardFormatAvailable(include).is_ok()
            && bytes(include).ok().is_some_and(|b| {
                b.len() >= 4 && u32::from_le_bytes(b[..4].try_into().unwrap_or_default()) == 0
            })
    }
}
pub enum Content {
    Files(Vec<String>),
    Text(String),
    Image(Vec<u8>),
}
pub fn read(respect_privacy: bool) -> Result<Content, String> {
    let _guard = open()?;
    if respect_privacy && private() {
        return Err("Clipboard source requested privacy; this copy was not captured".into());
    }
    // SAFETY: Clipboard remains open throughout each bounded format read.
    unsafe {
        if IsClipboardFormatAvailable(15).is_ok() {
            let handle = GetClipboardData(15).map_err(|e| e.to_string())?;
            let drop = HDROP(handle.0);
            let count = DragQueryFileW(drop, u32::MAX, None);
            let mut paths = Vec::new();
            for index in 0..count.min(100) {
                let size = DragQueryFileW(drop, index, None);
                let mut buffer = vec![0u16; size as usize + 1];
                DragQueryFileW(drop, index, Some(&mut buffer));
                paths.push(String::from_utf16_lossy(&buffer[..size as usize]));
            }
            if !paths.is_empty() {
                return Ok(Content::Files(paths));
            }
        }
        let png = format(&w!("PNG"));
        if IsClipboardFormatAvailable(png).is_ok() {
            return Ok(Content::Image(bytes(png)?));
        }
        for dib in [17, 8] {
            if IsClipboardFormatAvailable(dib).is_ok() {
                let data = bytes(dib)?;
                if data.len() >= 40 {
                    let header =
                        u32::from_le_bytes(data[0..4].try_into().unwrap_or_default()) as usize;
                    let depth = u16::from_le_bytes(data[14..16].try_into().unwrap_or_default());
                    let colors =
                        u32::from_le_bytes(data[32..36].try_into().unwrap_or_default()) as usize;
                    let compression =
                        u32::from_le_bytes(data[16..20].try_into().unwrap_or_default());
                    let palette = if colors > 0 {
                        colors * 4
                    } else if depth <= 8 {
                        (1usize << depth) * 4
                    } else {
                        0
                    };
                    let masks = if header == 40 && compression == 3 {
                        12
                    } else {
                        0
                    };
                    let offset = 14 + header + palette + masks;
                    if offset > data.len() + 14 {
                        return Err("Unsupported clipboard bitmap".into());
                    }
                    let mut bitmap = Vec::with_capacity(data.len() + 14);
                    bitmap.extend_from_slice(b"BM");
                    bitmap.extend_from_slice(&((data.len() + 14) as u32).to_le_bytes());
                    bitmap.extend_from_slice(&[0; 4]);
                    bitmap.extend_from_slice(&(offset as u32).to_le_bytes());
                    bitmap.extend_from_slice(&data);
                    return Ok(Content::Image(bitmap));
                }
            }
        }
        if IsClipboardFormatAvailable(13).is_ok() {
            let data = bytes(13)?;
            let text = data
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .take_while(|c| *c != 0)
                .collect::<Vec<_>>();
            return Ok(Content::Text(String::from_utf16_lossy(&text)));
        }
    }
    Err("Clipboard has no supported text, files or image".into())
}
pub fn write_text(owner: usize, text: &str) -> Result<(), String> {
    let _guard = open_write(owner)?;
    let data = text
        .encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    // SAFETY: Clipboard is owned by our live HWND and global memory ownership is
    // transferred by put. Sequence is recorded before closing to prevent loops.
    unsafe {
        EmptyClipboard().map_err(|e| e.to_string())?;
    }
    put(13, &data)?;
    OWN_SEQUENCE.store(sequence(), Ordering::Release);
    Ok(())
}
pub fn write_files(owner: usize, paths: &[PathBuf]) -> Result<(), String> {
    let _guard = open_write(owner)?;
    let mut data = Vec::new();
    data.extend_from_slice(&20u32.to_le_bytes());
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&1u32.to_le_bytes());
    for path in paths {
        for unit in path.to_string_lossy().encode_utf16().chain([0]) {
            data.extend_from_slice(&unit.to_le_bytes());
        }
    }
    data.extend_from_slice(&0u16.to_le_bytes());
    // SAFETY: DROPFILES has a 20-byte header followed by a double-NUL terminated
    // UTF-16 path list, copied into owned Windows global memory by put.
    unsafe {
        EmptyClipboard().map_err(|e| e.to_string())?;
    }
    put(15, &data)?;
    OWN_SEQUENCE.store(sequence(), Ordering::Release);
    Ok(())
}
pub fn write_image(owner: usize, path: &std::path::Path) -> Result<(), String> {
    let rgba = image::open(path).map_err(|e| e.to_string())?.to_rgba8();
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(rgba.clone())
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    let mut dib = vec![0u8; 40];
    dib[0..4].copy_from_slice(&40u32.to_le_bytes());
    dib[4..8].copy_from_slice(&(rgba.width() as i32).to_le_bytes());
    dib[8..12].copy_from_slice(&(-(rgba.height() as i32)).to_le_bytes());
    dib[12..14].copy_from_slice(&1u16.to_le_bytes());
    dib[14..16].copy_from_slice(&32u16.to_le_bytes());
    for p in rgba.pixels() {
        dib.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
    }
    let _guard = open_write(owner)?;
    // SAFETY: Both registered PNG and standard CF_DIB are owned memory formats;
    // providing both supports applications which implement either clipboard path.
    unsafe {
        EmptyClipboard().map_err(|e| e.to_string())?;
    }
    put(format(&w!("PNG")), png.get_ref())?;
    put(8, &dib)?;
    OWN_SEQUENCE.store(sequence(), Ordering::Release);
    Ok(())
}
pub fn selected_text(owner: usize) -> Result<String, String> {
    let (snapshot, original) = {
        let _guard = open()?;
        if private() {
            return Err(
                "Clipboard privacy protection prevents selection capture; paste the word instead"
                    .into(),
            );
        }
        let mut formats = Vec::new();
        let mut current = 0;
        // SAFETY: Enumerates only the currently open clipboard. A non-global or
        // oversized format aborts before Ctrl+C rather than losing original data.
        unsafe {
            loop {
                current = EnumClipboardFormats(current);
                if current == 0 {
                    break;
                }
                formats.push((current, bytes(current)?));
                if formats.len() > 100 {
                    return Err("Clipboard has too many formats to preserve".into());
                }
            }
        }
        (formats, sequence())
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    let key = |vk, flags| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                dwFlags: flags,
                ..Default::default()
            },
        },
    };
    // SAFETY: Sends only the explicit user-requested Ctrl+C chord to the current
    // foreground app; no clipboard restoration occurs over a newer sequence.
    let sent = unsafe {
        SendInput(
            &[
                key(VK_CONTROL, KEYBD_EVENT_FLAGS(0)),
                key(VIRTUAL_KEY(0x43), KEYBD_EVENT_FLAGS(0)),
                key(VIRTUAL_KEY(0x43), KEYEVENTF_KEYUP),
                key(VK_CONTROL, KEYEVENTF_KEYUP),
            ],
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    if sent != 4 {
        return Err("Windows could not copy the selection".into());
    }
    for _ in 0..25 {
        if sequence() != original {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    if sequence() == original {
        return Err("No selected text was copied. Select a word and try again.".into());
    }
    let copied = sequence();
    let result = match read(false) {
        Ok(Content::Text(text)) => Ok(text),
        Ok(_) => Err("The selected content is not text".into()),
        Err(error) => Err(error),
    };
    let _guard = open_write(owner)?;
    if sequence() == copied {
        // SAFETY: Recheck occurs while holding clipboard ownership, so another
        // app cannot modify it between the sequence comparison and restoration.
        unsafe {
            EmptyClipboard().map_err(|e| e.to_string())?;
        }
        for (format, data) in snapshot {
            put(format, &data)?;
        }
        OWN_SEQUENCE.store(sequence(), Ordering::Release);
    }
    result
}
pub fn start(app: tauri::AppHandle, core: crate::core::Core) {
    use tauri::{Emitter, Manager};
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(100));
        let mut screenshot = false;
        if let Ok(mut capture) = core.0.capture.lock() {
            if let Some((_, start)) = *capture {
                if start.elapsed() > Duration::from_secs(30) {
                    *capture = None;
                    app.state::<crate::window::WindowManager>()
                        .suppress_visibility(false);
                    let _ = app.emit("capture://state", false);
                } else {
                    screenshot = true;
                }
            }
        }
        if !UPDATED.swap(false, Ordering::AcqRel) {
            continue;
        }
        let seq = sequence();
        if seq == OWN_SEQUENCE.load(Ordering::Acquire) {
            continue;
        }
        let history = core.settings().is_ok_and(|s| s.clipboard_history);
        if !history && !screenshot {
            continue;
        }
        let content = match read(true) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let captured = match content {
            Content::Image(bytes) => {
                if screenshot {
                    let valid = core
                        .0
                        .capture
                        .lock()
                        .ok()
                        .and_then(|c| *c)
                        .is_some_and(|(initial, _)| seq != initial);
                    if !valid {
                        continue;
                    }
                }
                core.shelf_image(&bytes, !screenshot)
            }
            Content::Text(text) if !screenshot => core.shelf_text(text, true),
            Content::Files(paths) if !screenshot => core
                .shelf_files(paths, true)
                .and_then(|mut v| v.pop().ok_or("No files copied".into())),
            _ => continue,
        };
        if let Ok(item) = captured {
            if screenshot {
                if let Ok(mut state) = core.0.capture.lock() {
                    *state = None;
                }
                app.state::<crate::window::WindowManager>()
                    .suppress_visibility(false);
                let _ = app.emit("capture://state", false);
            }
            let _ = app.emit("shelf://item-added", &item);
        }
    });
}
