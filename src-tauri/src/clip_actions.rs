//! Commands behind the smart clipboard actions. Detection runs in the
//! frontend; these only write text back and open a link, with no network use.

use tauri::WebviewWindow;

const OPENABLE: [&str; 4] = ["http", "https", "mailto", "tel"];

/// Only web, mail and phone links may reach ShellExecute, so a crafted copy
/// can't launch a file or another protocol handler.
fn openable(target: &str) -> Result<&str, String> {
    let target = target.trim();
    let scheme = target
        .split_once(':')
        .map(|(s, _)| s.to_ascii_lowercase())
        .ok_or("Not a link")?;
    if !OPENABLE.contains(&scheme.as_str()) {
        return Err("Isle opens only web, email and phone links".into());
    }
    if target.len() > 4096 || target.chars().any(char::is_control) {
        return Err("This link is too long or malformed to open".into());
    }
    Ok(target)
}

/// Writes text without it landing in clipboard history as a new copy.
#[tauri::command]
pub fn clip_write_text(window: WebviewWindow, text: String) -> Result<(), String> {
    if text.is_empty() {
        return Err("Nothing to copy".into());
    }
    let owner = window.hwnd().map_err(|e| e.to_string())?.0 as usize;
    crate::clipboard::write_text(owner, &text)
}

#[tauri::command]
pub fn clip_open_url(url: String) -> Result<(), String> {
    crate::platform::shell_open(openable(&url)?)
}

#[cfg(test)]
mod tests {
    use super::openable;

    #[test]
    fn opens_only_web_mail_and_phone_links() {
        assert!(openable("https://example.com/a?b=1").is_ok());
        assert!(openable("mailto:me@example.com").is_ok());
        assert!(openable("TEL:+15551234567").is_ok());
        assert!(openable("file:///C:/Windows/System32/calc.exe").is_err());
        assert!(openable("C:\\Windows\\notepad.exe").is_err());
        assert!(openable("ms-settings:privacy").is_err());
        assert!(openable("https://example.com/\nother").is_err());
        assert!(openable("notalink").is_err());
    }
}
