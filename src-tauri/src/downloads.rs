use crate::core::{id, Core, Download};
use notify::{RecursiveMode, Watcher};
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};
pub fn start(app: AppHandle, core: Core) {
    std::thread::spawn(move || {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut watcher = match notify::recommended_watcher(move |event| {
            let _ = sender.send(event);
        }) {
            Ok(w) => w,
            Err(e) => {
                let _ = app.emit("download://error", e.to_string());
                return;
            }
        };
        let mut watched: Option<PathBuf> = None;
        let mut failed: Option<(PathBuf, Instant)> = None;
        let mut candidates: HashMap<PathBuf, (u64, Instant)> = HashMap::new();
        let mut proposed: HashMap<PathBuf, (u64, std::time::SystemTime)> = HashMap::new();
        loop {
            let settings = match core.settings() {
                Ok(s) => s,
                Err(_) => return,
            };
            let wanted = if settings.watch_downloads {
                Some(PathBuf::from(&settings.downloads_folder))
            } else {
                None
            };
            let retry_ready = failed.as_ref().is_none_or(|(path, since)| {
                wanted.as_ref() != Some(path) || since.elapsed() >= Duration::from_secs(30)
            });
            if wanted != watched && retry_ready {
                if let Some(old) = watched.take() {
                    let _ = watcher.unwatch(&old);
                }
                candidates.clear();
                if let Some(path) = wanted {
                    match watcher.watch(&path, RecursiveMode::NonRecursive) {
                        Ok(()) => {
                            watched = Some(path);
                            failed = None;
                        }
                        Err(e) => {
                            failed = Some((path, Instant::now()));
                            let _ = app.emit(
                                "download://error",
                                format!("Downloads folder cannot be watched: {e}"),
                            );
                        }
                    }
                }
            }
            if let Ok(Ok(event)) = receiver.recv_timeout(Duration::from_millis(250)) {
                for path in event.paths {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_lowercase())
                        .unwrap_or_default();
                    if ![".crdownload", ".tmp", ".part", ".partial"]
                        .iter()
                        .any(|suffix| name.ends_with(suffix))
                        && path.is_file()
                    {
                        if let Ok(meta) = std::fs::metadata(&path) {
                            candidates.insert(path, (meta.len(), Instant::now()));
                        }
                    }
                }
            }
            let mut ready = Vec::new();
            for (path, (size, since)) in &mut candidates {
                if let Ok(meta) = std::fs::metadata(path) {
                    if meta.len() != *size {
                        *size = meta.len();
                        *since = Instant::now();
                    } else if since.elapsed() >= Duration::from_secs(1)
                        && std::fs::File::open(path).is_ok()
                    {
                        let modified = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
                        if proposed.get(path) != Some(&(*size, modified)) {
                            proposed.insert(path.clone(), (*size, modified));
                            ready.push((path.clone(), *size));
                        }
                    }
                }
            }
            for (path, size) in ready {
                candidates.remove(&path);
                if settings.auto_add_downloads {
                    if let Ok(items) = core.shelf_files(vec![path.to_string_lossy().into()], false)
                    {
                        for item in items {
                            let _ = app.emit("shelf://item-added", item);
                        }
                    }
                } else {
                    let item = Download {
                        id: id(),
                        name: path
                            .file_name()
                            .map(|n| n.to_string_lossy().into())
                            .unwrap_or_default(),
                        path: path.to_string_lossy().into(),
                        size,
                    };
                    if let Ok(mut pending) = core.0.downloads.lock() {
                        pending.push(item.clone());
                        if pending.len() > 20 {
                            pending.remove(0);
                        }
                    }
                    let _ = app.emit("download://detected", item);
                }
            }
        }
    });
}
