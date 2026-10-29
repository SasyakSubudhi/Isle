//! AI usage: Claude Code and Codex limits, read from the session logs those
//! tools write on this PC. Nothing is sent anywhere and no sign-in is needed.
//! Chats in the Claude and ChatGPT apps or websites leave no such logs.
pub mod parse;

use crate::core::{now, Core};
use chrono::{Local, TimeZone};
use parse::{ClaudeLine, ClaudeUsage, CodexUsage, DAY};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};
use tauri::{AppHandle, Emitter, State};

const POLL: Duration = Duration::from_secs(90);
/// Logs older than this cannot change the last seven days.
const MAX_AGE: Duration = Duration::from_secs(8 * 24 * 3600);
const WARN_PERCENT: f64 = 80.0;
const WARNED_KEY: &str = "ai_usage:warned";

#[derive(Serialize, Clone, Default, PartialEq)]
pub struct AiUsage {
    pub checked_at: i64,
    /// None when Claude Code has never run on this PC.
    pub claude: Option<ClaudeUsage>,
    /// None when Codex has never reported limits on this PC.
    pub codex: Option<CodexUsage>,
}

#[derive(Default)]
struct Cache {
    /// Parsed Claude lines per log file, reused while the file is unchanged.
    claude: HashMap<PathBuf, (SystemTime, u64, Vec<ClaudeLine>)>,
    latest: Option<AiUsage>,
}
#[derive(Clone, Default)]
pub struct AiState(Arc<Mutex<Cache>>);

fn home() -> PathBuf {
    PathBuf::from(
        std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .unwrap_or_default(),
    )
}
fn claude_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(|v| {
            std::env::split_paths(&v)
                .map(|p| p.join("projects"))
                .collect()
        })
        .unwrap_or_default();
    roots.push(home().join(".claude").join("projects"));
    roots.push(home().join(".config").join("claude").join("projects"));
    roots.dedup();
    roots
}
fn codex_root() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".codex"))
        .join("sessions")
}

/// Recent `.jsonl` files under `root`, newest first.
fn recent_logs(root: &Path) -> Vec<(PathBuf, SystemTime, u64)> {
    let mut found = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0)];
    while let Some((dir, depth)) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            let path = entry.path();
            if meta.is_dir() {
                if depth < 5 {
                    stack.push((path, depth + 1));
                }
            } else if path.extension().is_some_and(|e| e == "jsonl") {
                let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                if modified.elapsed().is_ok_and(|age| age < MAX_AGE) {
                    found.push((path, modified, meta.len()));
                }
            }
        }
    }
    found.sort_by_key(|f| std::cmp::Reverse(f.1));
    found
}

fn lines(path: &Path) -> impl Iterator<Item = String> {
    fs::File::open(path)
        .ok()
        .into_iter()
        .flat_map(|f| BufReader::new(f).lines().map_while(Result::ok))
}

fn local_midnight(now: i64) -> i64 {
    let today = Local
        .timestamp_millis_opt(now)
        .single()
        .map(|t| t.date_naive())
        .unwrap_or_default();
    today
        .and_hms_opt(0, 0, 0)
        .and_then(|t| Local.from_local_datetime(&t).earliest())
        .map(|t| t.timestamp_millis())
        .unwrap_or(now - now.rem_euclid(DAY))
}

fn read_claude(cache: &mut Cache, now: i64) -> Option<ClaudeUsage> {
    let roots: Vec<PathBuf> = claude_roots().into_iter().filter(|r| r.is_dir()).collect();
    if roots.is_empty() {
        return None;
    }
    let files: Vec<_> = roots.iter().flat_map(|r| recent_logs(r)).collect();
    cache
        .claude
        .retain(|path, _| files.iter().any(|f| &f.0 == path));
    let mut all = Vec::new();
    for (path, modified, len) in files {
        let fresh = cache
            .claude
            .get(&path)
            .is_some_and(|(m, l, _)| *m == modified && *l == len);
        if !fresh {
            let parsed: Vec<ClaudeLine> = lines(&path)
                .filter_map(|l| parse::claude_line(&l))
                .collect();
            cache.claude.insert(path.clone(), (modified, len, parsed));
        }
        all.extend(cache.claude[&path].2.iter().cloned());
    }
    Some(parse::claude_summary(&all, now, local_midnight(now)))
}

fn read_codex(now: i64) -> Option<CodexUsage> {
    // The newest session with a limits report wins; older ones are stale.
    recent_logs(&codex_root())
        .into_iter()
        .take(20)
        .find_map(|(path, _, _)| {
            lines(&path)
                .filter_map(|l| parse::codex_line(&l))
                .last()
                .map(|u| parse::codex_now(u, now))
        })
}

fn refresh(state: &AiState) -> AiUsage {
    let now = now();
    let mut cache = state.0.lock().unwrap_or_else(|e| e.into_inner());
    let usage = AiUsage {
        checked_at: now,
        claude: read_claude(&mut cache, now),
        codex: read_codex(now),
    };
    cache.latest = Some(usage.clone());
    usage
}

/// Short notices for limits worth knowing about, each keyed so it shows once.
fn warnings(usage: &AiUsage, now: i64) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let at = |ms: i64| {
        Local
            .timestamp_millis_opt(ms)
            .single()
            .map(|t| t.format("%-I:%M %p").to_string())
            .unwrap_or_default()
    };
    if let Some(limit) = usage.claude.as_ref().and_then(|c| c.limit.as_ref()) {
        if limit.rejected && now - limit.at < 15 * 60_000 {
            out.push((
                format!("claude:{}", limit.resets_at),
                format!("Claude limit reached · resets {}", at(limit.resets_at)),
            ));
        }
    }
    if let Some(codex) = usage
        .codex
        .as_ref()
        .filter(|c| now - c.updated_at < 15 * 60_000)
    {
        for w in &codex.windows {
            if w.used_percent >= WARN_PERCENT {
                let full = w.used_percent >= 100.0;
                out.push((
                    format!("codex:{}:{}:{}", w.minutes, w.resets_at, full),
                    if full {
                        format!("Codex limit reached · resets {}", at(w.resets_at))
                    } else {
                        format!(
                            "Codex {}% used · resets {}",
                            w.used_percent.round(),
                            at(w.resets_at)
                        )
                    },
                ));
            }
        }
    }
    out
}

pub fn start(app: AppHandle, core: Core, state: AiState) {
    std::thread::spawn(move || loop {
        let before = state.0.lock().ok().and_then(|c| c.latest.clone());
        let usage = refresh(&state);
        if before.as_ref().map(|b| (&b.claude, &b.codex)) != Some((&usage.claude, &usage.codex)) {
            let _ = app.emit("ai://changed", &usage);
        }
        let shown = core
            .settings()
            .is_ok_and(|s| !s.hidden_tabs.iter().any(|t| t == "ai"));
        if shown {
            let mut warned: Vec<String> =
                core.kv_get(WARNED_KEY).ok().flatten().unwrap_or_default();
            for (key, title) in warnings(&usage, usage.checked_at) {
                if !warned.contains(&key) {
                    let _ = app.emit("ai://warning", title);
                    warned.push(key);
                }
            }
            let keep = warned.len().saturating_sub(20);
            let _ = core.kv_set(WARNED_KEY, &warned[keep..].to_vec());
        }
        std::thread::sleep(POLL);
    });
}

#[tauri::command]
pub async fn ai_usage(state: State<'_, AiState>, fresh: Option<bool>) -> Result<AiUsage, String> {
    let state = state.inner().clone();
    if !fresh.unwrap_or(false) {
        if let Some(latest) = state.0.lock().ok().and_then(|c| c.latest.clone()) {
            return Ok(latest);
        }
    }
    // Reading logs can take a moment, so keep it off the UI thread.
    tauri::async_runtime::spawn_blocking(move || refresh(&state))
        .await
        .map_err(|e| e.to_string())
}
