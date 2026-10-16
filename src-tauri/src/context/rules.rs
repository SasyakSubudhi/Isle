//! Pure rules for the context-aware pill. No Windows or crate dependencies, so
//! `rustc --test src-tauri/src/context/rules.rs` runs these tests on any OS.
//!
//! Isle only ever looks at the foreground app's executable name. Window titles
//! and content are never read for this feature.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Code,
    Meeting,
    Reading,
    Writing,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Code => "code",
            Kind::Meeting => "meeting",
            Kind::Reading => "reading",
            Kind::Writing => "writing",
        }
    }
}

const CODE: &[&str] = &[
    "code",
    "code - insiders",
    "codium",
    "cursor",
    "windsurf",
    "zed",
    "devenv",
    "idea64",
    "idea",
    "pycharm64",
    "webstorm64",
    "clion64",
    "rider64",
    "goland64",
    "rustrover64",
    "phpstorm64",
    "datagrip64",
    "studio64",
    "sublime_text",
    "notepad++",
    "nvim-qt",
    "windowsterminal",
    "wezterm-gui",
    "alacritty",
];
const MEETING: &[&str] = &[
    "zoom",
    "ms-teams",
    "msteams",
    "teams",
    "webex",
    "webexhost",
    "ciscocollabhost",
    "skype",
    "gotomeeting",
];
const READING: &[&str] = &[
    "chrome",
    "msedge",
    "firefox",
    "brave",
    "opera",
    "vivaldi",
    "arc",
    "acrobat",
    "acrord32",
    "sumatrapdf",
    "foxitpdfreader",
    "kindle",
];
const WRITING: &[&str] = &["winword", "obsidian", "notion", "onenote", "typora"];

/// Classifies an executable file stem, such as `Code` or `ms-teams`.
pub fn classify(exe_stem: &str) -> Option<Kind> {
    let name = exe_stem.trim().to_ascii_lowercase();
    let name = name.strip_suffix(".exe").unwrap_or(&name);
    [
        (CODE, Kind::Code),
        (MEETING, Kind::Meeting),
        (READING, Kind::Reading),
        (WRITING, Kind::Writing),
    ]
    .into_iter()
    .find(|(list, _)| list.contains(&name))
    .map(|(_, kind)| kind)
}

/// What the pill should show right now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Active {
    pub kind: Kind,
    /// The executable stem as Windows reports it, for the tooltip.
    pub app: String,
    /// Milliseconds since the epoch when this stretch of the activity began.
    pub since: i64,
}

/// A short trip to another app (checking chat, a file) keeps the running
/// timer when you come back within this window.
pub const GRACE_MS: i64 = 2 * 60 * 1000;

#[derive(Default)]
pub struct Tracker {
    shown: Option<Active>,
    /// The last classified activity and when it was last in the foreground.
    memory: Option<(Kind, i64, i64)>,
}

impl Tracker {
    /// Feeds one sample. `exe` is the foreground executable stem, `None` when
    /// it can't be read or the session is locked. `own_window` is true when
    /// Isle itself is in front. Returns the
    /// new state only when it changed.
    pub fn step(
        &mut self,
        exe: Option<&str>,
        own_window: bool,
        now: i64,
    ) -> Option<Option<Active>> {
        if own_window {
            // Clicking the pill shouldn't end a coding session.
            return None;
        }
        let next = exe.and_then(|exe| {
            let kind = classify(exe)?;
            let since = match self.memory {
                Some((last, since, seen)) if last == kind && now - seen <= GRACE_MS => since,
                _ => now,
            };
            self.memory = Some((kind, since, now));
            Some(Active {
                kind,
                app: exe.to_string(),
                since,
            })
        });
        if next == self.shown {
            None
        } else {
            self.shown = next.clone();
            Some(next)
        }
    }

    pub fn current(&self) -> Option<Active> {
        self.shown.clone()
    }

    pub fn reset(&mut self) -> bool {
        self.memory = None;
        self.shown.take().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_by_executable_name_only() {
        assert_eq!(classify("Code"), Some(Kind::Code));
        assert_eq!(classify("Code.exe"), Some(Kind::Code));
        assert_eq!(classify("Zoom"), Some(Kind::Meeting));
        assert_eq!(classify("ms-teams"), Some(Kind::Meeting));
        assert_eq!(classify("msedge"), Some(Kind::Reading));
        assert_eq!(classify("WINWORD"), Some(Kind::Writing));
        assert_eq!(classify("explorer"), None);
        assert_eq!(classify(""), None);
    }

    #[test]
    fn a_short_detour_keeps_the_timer() {
        let mut t = Tracker::default();
        assert_eq!(
            t.step(Some("Code"), false, 1_000),
            Some(Some(Active {
                kind: Kind::Code,
                app: "Code".into(),
                since: 1_000
            }))
        );
        // Same app a second later: nothing new to tell the pill.
        assert_eq!(t.step(Some("Code"), false, 2_000), None);
        // Over to Explorer: the pill goes back to idle…
        assert_eq!(t.step(Some("explorer"), false, 3_000), Some(None));
        // …and coming back within the grace window resumes the same stretch.
        let back = t.step(Some("Code"), false, 60_000).unwrap().unwrap();
        assert_eq!(back.since, 1_000);
    }

    #[test]
    fn a_long_break_or_a_different_activity_restarts() {
        let mut t = Tracker::default();
        t.step(Some("Code"), false, 0);
        t.step(None, false, 1_000);
        let later = t
            .step(Some("Code"), false, GRACE_MS + 10_000)
            .unwrap()
            .unwrap();
        assert_eq!(later.since, GRACE_MS + 10_000);
        let meeting = t
            .step(Some("Zoom"), false, GRACE_MS + 20_000)
            .unwrap()
            .unwrap();
        assert_eq!(meeting.kind, Kind::Meeting);
        assert_eq!(meeting.since, GRACE_MS + 20_000);
    }

    #[test]
    fn focusing_isle_itself_changes_nothing() {
        let mut t = Tracker::default();
        t.step(Some("Code"), false, 0);
        assert_eq!(t.step(None, true, 5_000), None);
        assert_eq!(t.current().map(|a| a.kind), Some(Kind::Code));
    }

    #[test]
    fn switching_editors_keeps_the_coding_stretch() {
        let mut t = Tracker::default();
        t.step(Some("Code"), false, 0);
        let terminal = t
            .step(Some("WindowsTerminal"), false, 9_000)
            .unwrap()
            .unwrap();
        assert_eq!(terminal.since, 0);
        assert_eq!(terminal.app, "WindowsTerminal");
    }
}
