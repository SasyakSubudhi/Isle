//! Reads Claude Code and Codex session logs into usage summaries. Pure: no files,
//! no clock, so it can be checked against sample lines. All times are Unix ms.
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;

pub const HOUR: i64 = 3_600_000;
pub const DAY: i64 = 24 * HOUR;
/// Claude's usage limits run in five-hour windows.
pub const WINDOW: i64 = 5 * HOUR;

/// One line of a Claude Code log that matters here.
#[derive(Debug, Clone, PartialEq)]
pub enum ClaudeLine {
    /// A model reply with its token counts, keyed so repeated lines count once.
    Usage { key: String, at: i64, tokens: u64 },
    /// Claude refused or warned because of a usage limit; `resets_at` is exact.
    Limit {
        at: i64,
        kind: String,
        resets_at: i64,
        rejected: bool,
    },
}

fn time(value: &Value) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value.as_str()?)
        .ok()
        .map(|t| t.timestamp_millis())
}
fn num(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}

pub fn claude_line(line: &str) -> Option<ClaudeLine> {
    if !line.contains("\"usage\"") && !line.contains("resetsAt") {
        return None;
    }
    let row: Value = serde_json::from_str(line).ok()?;
    let at = time(row.get("timestamp")?)?;
    let quota = row
        .get("quotaLimits")
        .or_else(|| row.pointer("/apiErrorParams/rate_limit_info"));
    if let Some(resets) = quota
        .and_then(|q| q.get("resetsAt"))
        .and_then(Value::as_i64)
    {
        let quota = quota.unwrap_or(&Value::Null);
        return Some(ClaudeLine::Limit {
            at,
            kind: quota
                .get("rateLimitType")
                .and_then(Value::as_str)
                .unwrap_or("five_hour")
                .to_string(),
            resets_at: resets * 1000,
            rejected: quota.get("status").and_then(Value::as_str) == Some("rejected"),
        });
    }
    let message = row.get("message")?;
    let usage = message.get("usage")?;
    // Cache reads are re-sent context, not new work, so they are left out.
    let tokens = num(usage, "input_tokens")
        + num(usage, "output_tokens")
        + num(usage, "cache_creation_input_tokens");
    if tokens == 0 {
        return None;
    }
    let key = format!(
        "{}:{}",
        message.get("id").and_then(Value::as_str).unwrap_or(""),
        row.get("requestId").and_then(Value::as_str).unwrap_or("")
    );
    Some(ClaudeLine::Usage { key, at, tokens })
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ClaudeWindow {
    pub start: i64,
    pub end: i64,
    pub tokens: u64,
    pub messages: u32,
    /// True when Claude itself reported when this window ends.
    pub exact_reset: bool,
}
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ClaudeLimit {
    pub kind: String,
    pub resets_at: i64,
    pub rejected: bool,
    pub at: i64,
}
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ClaudeUsage {
    pub last_used: Option<i64>,
    pub window: Option<ClaudeWindow>,
    pub today_tokens: u64,
    pub week_tokens: u64,
    /// Tokens per day for the last seven days, oldest first.
    pub days: Vec<u64>,
    /// A limit Claude reported that has not reset yet.
    pub limit: Option<ClaudeLimit>,
}

/// `day_start` is local midnight today, so "today" follows the PC's clock.
pub fn claude_summary(lines: &[ClaudeLine], now: i64, day_start: i64) -> ClaudeUsage {
    let mut seen = HashSet::new();
    let mut entries: Vec<(i64, u64)> = Vec::new();
    let mut limit: Option<ClaudeLimit> = None;
    for line in lines {
        match line {
            ClaudeLine::Usage { key, at, tokens } => {
                if seen.insert(key.as_str()) {
                    entries.push((*at, *tokens));
                }
            }
            ClaudeLine::Limit {
                at,
                kind,
                resets_at,
                rejected,
            } => {
                if *resets_at > now && limit.as_ref().is_none_or(|l| *at > l.at) {
                    limit = Some(ClaudeLimit {
                        kind: kind.clone(),
                        resets_at: *resets_at,
                        rejected: *rejected,
                        at: *at,
                    });
                }
            }
        }
    }
    entries.sort_unstable();
    let mut days = vec![0u64; 7];
    for (at, tokens) in &entries {
        if *at <= now && *at > day_start - 6 * DAY {
            let back = if *at >= day_start {
                0
            } else {
                ((day_start - at - 1) / DAY + 1) as usize
            };
            if back < 7 {
                days[6 - back] += tokens;
            }
        }
    }
    // Windows open on the hour of the first message and last five hours, the
    // way Claude's own limits roll; a quiet gap of five hours also starts anew.
    let mut window: Option<ClaudeWindow> = None;
    let mut last = i64::MIN;
    for (at, tokens) in &entries {
        let fresh = window
            .as_ref()
            .is_none_or(|w| *at >= w.end || at - last >= WINDOW);
        if fresh {
            let start = at - at.rem_euclid(HOUR);
            window = Some(ClaudeWindow {
                start,
                end: start + WINDOW,
                tokens: 0,
                messages: 0,
                exact_reset: false,
            });
        }
        let w = window.as_mut().expect("window was just set");
        w.tokens += tokens;
        w.messages += 1;
        last = *at;
    }
    let mut window = window.filter(|w| w.end > now);
    if let (Some(w), Some(l)) = (window.as_mut(), limit.as_ref()) {
        if l.kind == "five_hour" {
            w.end = l.resets_at;
            w.start = l.resets_at - WINDOW;
            w.exact_reset = true;
        }
    }
    ClaudeUsage {
        last_used: entries.last().map(|e| e.0),
        window,
        today_tokens: days[6],
        week_tokens: days.iter().sum(),
        days,
        limit,
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct CodexWindow {
    pub minutes: u32,
    pub used_percent: f64,
    pub resets_at: i64,
}
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct CodexUsage {
    pub plan: Option<String>,
    /// When Codex last reported its limits.
    pub updated_at: i64,
    pub windows: Vec<CodexWindow>,
    pub limit_reached: bool,
}

/// The rate limits from a Codex `token_count` event, as of that event.
pub fn codex_line(line: &str) -> Option<CodexUsage> {
    if !line.contains("\"token_count\"") || !line.contains("rate_limits") {
        return None;
    }
    let row: Value = serde_json::from_str(line).ok()?;
    let at = time(row.get("timestamp")?)?;
    let limits = row.pointer("/payload/rate_limits")?;
    let mut windows = Vec::new();
    for key in ["primary", "secondary"] {
        let Some(w) = limits.get(key).filter(|w| w.is_object()) else {
            continue;
        };
        let resets_at = w
            .get("resets_at")
            .and_then(Value::as_i64)
            .map(|s| s * 1000)
            .or_else(|| {
                w.get("resets_in_seconds")
                    .and_then(Value::as_i64)
                    .map(|s| at + s * 1000)
            });
        let (Some(percent), Some(resets_at)) =
            (w.get("used_percent").and_then(Value::as_f64), resets_at)
        else {
            continue;
        };
        windows.push(CodexWindow {
            minutes: w.get("window_minutes").and_then(Value::as_u64).unwrap_or(0) as u32,
            used_percent: percent.clamp(0.0, 100.0),
            resets_at,
        });
    }
    if windows.is_empty() {
        return None;
    }
    Some(CodexUsage {
        plan: limits
            .get("plan_type")
            .and_then(Value::as_str)
            .map(str::to_string),
        updated_at: at,
        windows,
        limit_reached: limits
            .get("rate_limit_reached_type")
            .is_some_and(|v| !v.is_null()),
    })
}

/// A window whose reset time has passed is back to zero, whatever the log said.
/// Its `resets_at` stays in the past: the next window only opens with use.
pub fn codex_now(mut usage: CodexUsage, now: i64) -> CodexUsage {
    for w in &mut usage.windows {
        if w.resets_at <= now {
            w.used_percent = 0.0;
            usage.limit_reached = false;
        }
    }
    usage
}

#[cfg(test)]
mod tests {
    use super::*;
    const T0: i64 = 1_792_998_000_000; // a whole hour, UTC

    fn usage(id: &str, at: i64, tokens: u64) -> ClaudeLine {
        ClaudeLine::Usage {
            key: id.into(),
            at,
            tokens,
        }
    }

    #[test]
    fn reads_claude_reply_and_limit_lines() {
        let reply = r#"{"timestamp":"2026-10-10T12:34:06.532Z","requestId":"r1","message":{"id":"m1","usage":{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":100,"cache_read_input_tokens":9000}}}"#;
        assert!(matches!(
            claude_line(reply),
            Some(ClaudeLine::Usage { tokens: 115, .. })
        ));
        let limit = r#"{"timestamp":"2026-10-10T12:34:06.532Z","message":{"usage":{"input_tokens":0,"output_tokens":0}},"quotaLimits":{"status":"rejected","resetsAt":1791648600,"rateLimitType":"five_hour"}}"#;
        assert_eq!(
            claude_line(limit),
            Some(ClaudeLine::Limit {
                at: 1_791_635_646_532,
                kind: "five_hour".into(),
                resets_at: 1_791_648_600_000,
                rejected: true
            })
        );
        assert_eq!(claude_line(r#"{"type":"user"}"#), None);
    }

    #[test]
    fn groups_claude_messages_into_five_hour_windows() {
        let lines = [
            usage("a", T0 - 10 * HOUR, 50),
            usage("b", T0 + 20 * 60_000, 100),
            usage("b", T0 + 20 * 60_000, 100), // the same reply logged twice
            usage("c", T0 + 3 * HOUR, 30),
        ];
        let s = claude_summary(&lines, T0 + 4 * HOUR, T0 - 12 * HOUR);
        let w = s.window.unwrap();
        assert_eq!(
            (w.start, w.end, w.tokens, w.messages),
            (T0, T0 + WINDOW, 130, 2)
        );
        assert!(!w.exact_reset);
        assert_eq!(s.today_tokens, 180);
        // After the window ends there is no current window.
        assert!(claude_summary(&lines, T0 + 6 * HOUR, T0).window.is_none());
    }

    #[test]
    fn a_reported_limit_sets_the_exact_reset() {
        let lines = [
            usage("a", T0 + HOUR, 10),
            ClaudeLine::Limit {
                at: T0 + 2 * HOUR,
                kind: "five_hour".into(),
                resets_at: T0 + 4 * HOUR + 1_800_000,
                rejected: true,
            },
        ];
        let s = claude_summary(&lines, T0 + 3 * HOUR, T0);
        assert_eq!(s.window.unwrap().end, T0 + 4 * HOUR + 1_800_000);
        assert!(s.limit.unwrap().rejected);
        // Once the reset passes, the limit is gone.
        assert!(claude_summary(&lines, T0 + 5 * HOUR, T0).limit.is_none());
    }

    #[test]
    fn reads_codex_rate_limits() {
        let line = r#"{"timestamp":"2026-10-28T18:15:07.560Z","type":"event_msg","payload":{"type":"token_count","rate_limits":{"primary":{"used_percent":29.0,"window_minutes":300,"resets_at":1793212699},"secondary":{"used_percent":4.0,"window_minutes":10080,"resets_at":1793799499},"plan_type":"plus","rate_limit_reached_type":null}}}"#;
        let u = codex_line(line).unwrap();
        assert_eq!(u.plan.as_deref(), Some("plus"));
        assert_eq!(u.windows.len(), 2);
        assert_eq!(u.windows[0].used_percent, 29.0);
        assert!(!u.limit_reached);
        let later = codex_now(u, 1_793_212_699_000 + 60_000);
        assert_eq!(later.windows[0].used_percent, 0.0);
        assert_eq!(later.windows[1].used_percent, 4.0);
    }
}
