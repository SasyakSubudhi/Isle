pub mod accent;

use serde::Serialize;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{AppHandle, Emitter};
use windows::{
    Media::Control::{
        GlobalSystemMediaTransportControlsSessionManager as SessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as Playback,
    },
    Storage::Streams::DataReader,
};

#[derive(Clone, Debug, Serialize, Default, PartialEq)]
pub struct MediaState {
    pub available: bool,
    pub title: String,
    pub artist: String,
    pub source: String,
    pub playing: bool,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub artwork: Option<String>,
    /// Accent colour taken from the artwork, as `#rrggbb`.
    pub accent: Option<String>,
    pub error: Option<String>,
}
#[derive(Clone, Default)]
pub struct Media(pub Arc<Mutex<MediaState>>);
pub fn snapshot(manager: &SessionManager, cache: &std::path::Path) -> Result<MediaState, String> {
    let session = match manager.GetCurrentSession() {
        Ok(s) => s,
        Err(_) => return Ok(MediaState::default()),
    };
    let properties = session
        .TryGetMediaPropertiesAsync()
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|e| e.to_string())?;
    let title = properties.Title().map_err(|e| e.to_string())?.to_string();
    let artist = properties.Artist().map_err(|e| e.to_string())?.to_string();
    let source = session
        .SourceAppUserModelId()
        .map_err(|e| e.to_string())?
        .to_string();
    let info = session.GetPlaybackInfo().map_err(|e| e.to_string())?;
    let playing = info.PlaybackStatus().map_err(|e| e.to_string())? == Playback::Playing;
    let timeline = session.GetTimelineProperties().map_err(|e| e.to_string())?;
    let start = timeline.StartTime().map_err(|e| e.to_string())?.Duration;
    let position = timeline.Position().map_err(|e| e.to_string())?.Duration;
    let end = timeline.EndTime().map_err(|e| e.to_string())?.Duration;
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    (&title, &artist, &source).hash(&mut hash);
    let art = cache.join("media").join(format!("{:x}.png", hash.finish()));
    let artwork = if art.exists() {
        Some(art.to_string_lossy().to_string())
    } else {
        (clip_art(&properties).ok()).and_then(|bytes| {
            image::load_from_memory(&bytes)
                .ok()?
                .thumbnail(256, 256)
                .save(&art)
                .ok()?;
            Some(art.to_string_lossy().into())
        })
    };
    Ok(MediaState {
        available: true,
        title,
        artist,
        source,
        playing,
        position_seconds: (position - start).max(0) as f64 / 10_000_000.0,
        duration_seconds: (end - start).max(0) as f64 / 10_000_000.0,
        artwork,
        accent: None,
        error: None,
    })
}
fn clip_art(
    properties: &windows::Media::Control::GlobalSystemMediaTransportControlsSessionMediaProperties,
) -> Result<Vec<u8>, String> {
    let reference = properties.Thumbnail().map_err(|e| e.to_string())?;
    let stream = reference
        .OpenReadAsync()
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|e| e.to_string())?;
    let size = stream.Size().map_err(|e| e.to_string())?;
    if size == 0 || size > 8_000_000 {
        return Err("Album artwork is unavailable or too large".into());
    }
    let input = stream.GetInputStreamAt(0).map_err(|e| e.to_string())?;
    let reader = DataReader::CreateDataReader(&input).map_err(|e| e.to_string())?;
    reader
        .LoadAsync(size as u32)
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|e| e.to_string())?;
    let mut bytes = vec![0u8; size as usize];
    reader.ReadBytes(&mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes)
}
pub fn command(command: &str, position: Option<f64>) -> Result<(), String> {
    let manager = SessionManager::RequestAsync()
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|e| e.to_string())?;
    let session = manager
        .GetCurrentSession()
        .map_err(|_| "No media player is available")?;
    let operation = match command {
        "toggle" => session.TryTogglePlayPauseAsync(),
        "next" => session.TrySkipNextAsync(),
        "previous" => session.TrySkipPreviousAsync(),
        "seek" => {
            let seconds = position
                .filter(|p| p.is_finite() && *p >= 0.0)
                .ok_or("Invalid seek position")?;
            session.TryChangePlaybackPositionAsync((seconds * 10_000_000.0) as i64)
        }
        _ => return Err("Unknown media command".into()),
    };
    if !operation
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|e| e.to_string())?
    {
        return Err("The player did not accept this control".into());
    }
    Ok(())
}
fn artwork_accent(path: &str) -> Option<String> {
    let image = image::open(path).ok()?.thumbnail(64, 64).to_rgb8();
    accent::accent(image.pixels().map(|p| p.0))
}
pub fn start(app: AppHandle, media: Media, cache: std::path::PathBuf) {
    std::thread::spawn(move || {
        // SAFETY: The dedicated worker owns its MTA; WinRT interfaces never escape
        // the thread. Media command calls use their own initialized Tauri thread.
        unsafe {
            let _ = windows::Win32::System::WinRT::RoInitialize(
                windows::Win32::System::WinRT::RO_INIT_MULTITHREADED,
            );
        }
        let manager = SessionManager::RequestAsync().and_then(|op| op.join());
        let mut accents: (Option<String>, Option<String>) = (None, None);
        loop {
            let mut next = match &manager {
                Ok(manager) => snapshot(manager, &cache).unwrap_or_else(|error| MediaState {
                    error: Some(error),
                    ..Default::default()
                }),
                Err(error) => MediaState {
                    error: Some(error.to_string()),
                    ..Default::default()
                },
            };
            if next.artwork != accents.0 {
                accents = (
                    next.artwork.clone(),
                    next.artwork.as_deref().and_then(artwork_accent),
                );
            }
            next.accent = accents.1.clone();
            if let Ok(mut current) = media.0.lock() {
                if *current != next {
                    *current = next.clone();
                    let _ = app.emit("media://changed", &next);
                }
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}
