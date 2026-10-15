# Isle implementation contract

The user-approved plan and original brief govern this project. Windows 11 x64,
Tauri 2, React 18, TypeScript, Vite, Tailwind, Framer Motion, Zustand, Rust-owned
SQLite and documented Win32 APIs are fixed choices.

## Ordered gates

- M0: runnable transparent static pill; no taskbar entry.
- M1: taskbar placement, no-activate/click-through, fullscreen hiding, DPI/monitors.
- M2: island reducer, hotkeys, morphing, tabs, accessibility and design primitives.
- M3: real native shelf/clipboard, cleanup, stacks, notes and screenshots.
- M4: measured usage tracking, charts, exclusions and daily nudges.
- M5: dictionary, cache, selected-text lookup, offline and audio.
- M6: media, Chrome profile shortcuts, persistent Pomodoro, privacy mode/animations.
- M7: settings, tray, startup, recovery, packaging and fresh-VM/performance checks.

Do not advance past a failed or unavailable mandatory gate. Log verification
honestly in MILESTONES.md. Commit each passed milestone. Every milestone must
remain runnable with `npm run tauri dev`.

## Agreed defaults

Clipboard history is opt-in; explicit paste/drop/capture works without it.
References by default, optional shelf cache, 24-hour expiry, pins never expire.
Never delete source files. Windows snipping captures only Isle-initiated sessions.
Dictionary open and selected-text lookup are separate rebindable actions.
Chrome shortcuts share one explicitly chosen existing profile.
Pomodoro 25/5/15 minutes, long break after four focus sessions, manual transitions.
App nudges once per app per day. Presentation mode shows neutral pill/timer and
suppresses private previews; timers and usage tracking continue.
Idle fluid glow with reduced-motion support. No telemetry/cloud sync.
Codex quota and in-Isle grammar checking are deferred.
