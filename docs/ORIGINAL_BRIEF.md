# Project: "Isle" — a Dynamic Island for the Windows 11 Taskbar

> Hand this whole file to Codex. Build milestone by milestone. Do not move to the next milestone until the current one's acceptance criteria pass. Each milestone must leave the app in a runnable state (`npm run tauri dev`).

## 1. Product summary

A small pill that floats in the empty space of the Windows 11 taskbar. Idle, it is a quiet, tiny capsule. On hover, click, hotkey or event it morphs (spring animation) into an expanded panel. It is a multi-tool:

1. **Shelf**: a temporary clipboard / drop zone for files, text, images and links. Download a file from the browser, drop it on the island, later drag it out into WhatsApp Web/Desktop, Explorer, Gmail, etc.
2. **Quick Dictionary**: global hotkey, type a word, get meaning, phonetics, example.
3. **Screen Time**: per-app usage today/week, distribution donut, timeline.
4. **Now Playing**: media controls and track info from any player.
5. **Extras** (later): Pomodoro/timer, battery/volume/network glance, copy-history.

Primary quality bar: the animation and visual polish must feel like Apple's Dynamic Island. The app must actually run reliably, so prefer boring, proven techniques.

## 2. Tech stack (decided, do not substitute)

| Layer | Choice | Why |
|---|---|---|
| Shell | **Tauri 2** (Rust) | Tiny RAM/CPU footprint, WebView2 transparent windows, native Win32 access via the `windows` crate |
| Frontend | **React 18 + TypeScript + Vite** | Fast iteration |
| Animation | **Framer Motion** (layout animations + springs) | Best for morphing pill shapes |
| Styling | **Tailwind CSS** + CSS variables for theme | Consistent design tokens |
| Charts | **visx** or hand-written SVG (donut, bars) | Full control, small bundle |
| State | **Zustand** | Simple |
| Storage | **SQLite** via `rusqlite` (Rust side) | Screen time + shelf metadata |
| Win32 | `windows` crate (official Microsoft bindings) | Hooks, UI Automation, OLE drag-drop, GSMTC |

Windows 11 only (x64). Ship a portable `.exe` and an installer (`tauri build`).

## 3. Repository layout

```
isle/
  src/                      # React frontend
    app/                    # root, island state machine
    island/                 # IslandShell, Pill, ExpandedPanel, morph logic
    modules/
      shelf/                # ShelfView, ShelfItem, drag-out
      dictionary/           # DictionaryView
      screentime/           # ScreenTimeView, Donut, Timeline
      media/                # NowPlayingView
    ui/                     # design system primitives (Glass, Button, Chip, Tooltip)
    lib/                    # ipc.ts (typed invoke wrappers), hooks, springs.ts
    styles/                 # tokens.css, globals.css
  src-tauri/
    src/
      main.rs
      window/               # island window setup, taskbar positioning, z-order keeper
      taskbar/              # free-space detection (UI Automation)
      usage/                # foreground tracker, idle detection, sqlite
      shelf/                # file copy store, clipboard CF_HDROP, native drag source
      clipboard/            # clipboard listener
      media/                # GSMTC bridge
      dictionary/           # HTTP client + cache
      settings/             # JSON settings
      commands.rs           # all #[tauri::command]s
    tauri.conf.json
  PLAN.md
```

## 4. Core technical design

### 4.1 The island window (most important and riskiest part)

- A single Tauri window: `decorations: false`, `transparent: true`, `alwaysOnTop: true`, `skipTaskbar: true`, `shadow: false`, `resizable: false`, `focus: false`.
- Set extended styles via Win32 after creation: `WS_EX_TOOLWINDOW` (no Alt-Tab entry), `WS_EX_NOACTIVATE` in idle state (never steal focus from the app user is working in), `WS_EX_LAYERED` as Tauri needs. When the user needs to type (dictionary), temporarily remove `NOACTIVATE` and focus the input; restore on collapse.
- The OS window is **a fixed, larger transparent canvas** (e.g. 560×360) anchored at the taskbar free slot. The visible pill/panel is drawn *inside* it by React and animated with Framer Motion. This avoids janky native window resizing. Transparent regions must be **click-through**: use `SetWindowRgn`/hit-testing or toggle `setIgnoreCursorEvents(true)` and flip it to `false` only when the cursor is over the pill (the frontend reports pill bounds via `mousemove` + a Rust-side cursor poll at ~60 Hz while idle so hover can wake it even when click-through).
- Anchor point: collapsed pill sits in the taskbar; expanded panel grows **upward** out of the taskbar (since taskbar is at the bottom) so it stays on screen.
- Z-order: Windows 11 taskbar is topmost too. Call `SetWindowPos(HWND_TOPMOST, SWP_NOACTIVATE)` on a ~1 s timer and on `EVENT_SYSTEM_FOREGROUND`, and when `Shell_TrayWnd` z-order changes, so the island stays above the taskbar. Hide the island automatically when a fullscreen app/game is foreground (check `SHQueryUserNotificationState` = busy/D3D fullscreen/presentation mode).
- DPI: make the app Per-Monitor-V2 aware. Do all positioning in physical pixels and convert for the webview scale factor.
- Multi-monitor: show only on the primary monitor's taskbar by default; setting to choose monitor. Re-position on `WM_DISPLAYCHANGE`, `WM_SETTINGCHANGE` (taskbar moved/resized), and DPI changes.

### 4.2 Finding free space in the taskbar

1. Get taskbar rect with `SHAppBarMessage(ABM_GETTASKBARPOS)` (handles taskbar position/auto-hide).
2. Use **UI Automation** (`IUIAutomation`) on `Shell_TrayWnd` to enumerate taskbar buttons (Start, Search, Task View, pinned/running apps, tray overflow, clock) and collect their bounding rectangles.
3. Compute the gaps along the taskbar's main axis between consecutive elements; choose the largest gap that is at least `pillMinWidth + padding` (default pill 120 px). Prefer the gap between the app icons and the system tray (right side), fall back to the left widgets area.
4. Re-run detection when apps are opened/closed (icons change) — debounce 500 ms, animate the pill sliding to the new position.
5. **Fallback** if no gap is big enough or UIA fails: place the pill at top-center of the screen, and expose a "Position" setting (Taskbar free space / Top-center / Custom drag).
6. Don't try to inject into or modify Explorer. Overlay only.

### 4.3 Island state machine

States: `idle` (small pill, shows a tiny live glyph such as media waveform or a shelf count) → `peek` (medium pill with one-line info, on events like "download added to shelf", "timer done") → `expanded` (full panel with tabs) → back with timeouts.

Triggers:
- Hover > 150 ms or click → `expanded`; mouse leave > 600 ms → collapse (unless pinned).
- Global hotkeys (register via `tauri-plugin-global-shortcut`): `Ctrl+Alt+Space` toggle island, `Ctrl+Alt+D` open dictionary with input focused, `Ctrl+Alt+S` shelf, `Ctrl+Alt+T` screen time.
- Events: file dropped / new download / clipboard capture → `peek`.
- Esc collapses.

Implement with an explicit reducer (e.g. XState-lite or a Zustand reducer), not scattered booleans.

### 4.4 Shelf (temporary clipboard / file drop zone) — flagship feature

Goal flow: download file in Chrome → it lands on the island → later drag it from the island into WhatsApp (or press Ctrl+V there).

Inputs:
- **Drag & drop onto the island** from Explorer or from the browser's download bubble (HTML5 drop gives `File` objects only for some sources; use Tauri's `onDragDropEvent` to get real file paths). Dragging approaching the island auto-expands it into a "drop here" state (listen to drag-enter on the window; because of click-through, temporarily disable click-through while any drag is in progress by watching `GetAsyncKeyState(VK_LBUTTON)` + cursor position near the pill).
- **Downloads watcher** (optional, on by default): `notify` crate watching the user's Downloads folder; when a finished download appears (ignore `.crdownload`/`.tmp`/`.part`, wait until size stable for 1 s), show a `peek` with a "Add to shelf" button. Never auto-add silently without the setting enabled.
- **Clipboard capture**: listen with `AddClipboardFormatListener`. Capture text, URLs, images, and file lists (CF_HDROP) into a *Clipboard history* tab; items can be promoted to the shelf (pin).
- **Paste into island** (`Ctrl+V` while expanded) adds whatever is on the clipboard.

Storage:
- Files are **referenced, not moved**, by default; optional "copy to shelf cache" (`%LOCALAPPDATA%/Isle/shelf/`) so items survive the original being deleted. Metadata in SQLite: id, kind, name, path, size, mime, thumbnail path, created_at, pinned, expires_at.
- Items auto-expire (default 24 h, configurable, pinned never expire). Show a subtle ring/countdown on each chip.

Outputs (this is the part that must really work):
- **Native drag-out**: implement an OLE drag source in Rust (`DoDragDrop` with `IDataObject` carrying `CF_HDROP`, or use a maintained crate such as `tauri-plugin-drag` / `drag-rs`). Dragging a chip from the island must drop correctly into WhatsApp Desktop, WhatsApp Web (browser), Explorer, Discord, Gmail compose, etc.
- **Click to copy**: puts the file on the clipboard as `CF_HDROP` so `Ctrl+V` pastes the real file in WhatsApp. Text/images copy as text/PNG.
- Right-click menu on a chip: Open, Show in folder, Copy, Rename, Pin, Remove, Remove & delete cache.
- Thumbnails: images (resize via `image` crate), PDFs/videos/others use the shell icon/thumbnail via `IShellItemImageFactory`.

### 4.5 Quick Dictionary

- Hotkey opens expanded island on Dictionary tab with focused input (requires temporarily dropping `WS_EX_NOACTIVATE`).
- Free online source: `https://api.dictionaryapi.dev/api/v2/entries/en/<word>` (meaning, phonetic, audio URL, examples, synonyms). Cache results in SQLite; show cached instantly and work offline for previously looked-up words.
- Also support "select text anywhere → hotkey": simulate `Ctrl+C` (save/restore the clipboard around it), read the text, look it up.
- UI: word large, phonetic + play-audio button, part-of-speech chips, definitions list with examples, synonyms as clickable chips, "Save word" to a vocabulary list (SQLite).
- Debounced search-as-you-type with graceful empty/not-found/offline states.

### 4.6 Screen Time

Tracking (Rust background thread, zero-polling where possible):
- `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` for foreground changes plus a 1 s heartbeat to accumulate time.
- For the foreground window: PID via `GetWindowThreadProcessId`, exe path via `QueryFullProcessImageNameW`, friendly name from file version info (`ProductName`) with fallback to exe stem; window title optional (off by default for privacy). Handle UWP apps (`ApplicationFrameHost.exe`) by resolving the real child process / AUMID.
- **Idle detection**: `GetLastInputInfo`; stop counting after 2 min of no input (configurable). Also stop on lock screen (`WTSRegisterSessionNotification`) and sleep.
- Store sessions in SQLite: `sessions(id, app_id, start_ts, end_ts)`, `apps(id, exe_path, display_name, icon_png, color)`. Merge adjacent sessions; aggregate queries by day/week/hour.
- Extract icons with `SHGetFileInfo`/`ExtractIconEx` → PNG, cache; derive an accent color per app from the icon's dominant color.

UI:
- Header: total screen time today, delta vs yesterday.
- **Donut chart** of distribution by app (top 5 + "Other"), animated on open, legend with icons and durations.
- **Hourly bar timeline** for today (stacked by app), toggle Today / 7 days.
- Tap an app to see its trend. Option to exclude apps (e.g. Isle itself, lock screen) and optional daily limit notification per app (later).
- All data stays local. Provide "Clear data" and "Pause tracking" in settings.

### 4.7 Now Playing

- Use Windows **GSMTC** (`Windows.Media.Control.GlobalSystemMediaTransportControlsSessionManager`) to get title, artist, album art, playback status, timeline; commands: play/pause/next/previous/seek.
- Idle pill shows a small animated waveform + album-art dot while playing; expanded shows art, scrubber, controls. Dominant album-art color tints the island glow.

### 4.8 Extras (Milestone 7, only after everything else is solid)

Pomodoro/countdown timer with a ring in the pill; battery/volume/Wi‑Fi quick glance; "copy history" search; quick notes; calendar next-event (skip unless time allows).

## 5. Design spec ("really really good" frontend)

Visual language: **liquid-glass dark** capsule.
- Background: near-black `#0B0B0F` at ~85% opacity with `backdrop-filter: blur(24px) saturate(180%)` is NOT reliably available on a transparent Tauri window over desktop content. Therefore: render a solid-ish dark surface (`#0A0A0C`, 94–97% opaque) plus an inner 1 px top highlight (`inset 0 1px 0 rgba(255,255,255,.08)`) and a soft outer shadow; fake glass with subtle gradients and noise texture. Optionally enable Windows 11 **Mica/Acrylic** via the `window-vibrancy` crate on the pill region only if it renders cleanly; keep a solid fallback.
- Shape: fully rounded pill idle (height 32–36 px, width 120–160 px) → rounded rectangle expanded (radius 28 px, ~420×260 px, resizes per tab). Shape morph must be a *continuous* animation using Framer Motion `layout` + `borderRadius` spring (stiffness ~380, damping ~30, mass ~0.9). No cross-fades between two separate elements for the shell.
- Content choreography: content fades/blur-scales in with 40–60 ms stagger *after* the shell reaches ~70% size; on collapse content leaves first (faster), then shell shrinks.
- Typography: Inter / "Segoe UI Variable"; tabular numbers for times; sizes 11/13/15/22/34.
- Color: neutral grays + one accent that adapts to context (album art color, app color, green for shelf success). Accent glows as a soft 40 px blurred radial behind content, low opacity.
- Micro-interactions: hover lift on chips, press scale 0.97, drag ghost with slight tilt, shelf drop zone with a breathing dashed ring, success checkmark morph, shake on error, haptic-style bounce when an item lands.
- Tabs: icon-only segmented control with a sliding highlight (shared `layoutId`).
- Empty states with small illustrations (inline SVG), never blank panels.
- Accessibility: keyboard navigable, visible focus rings, `prefers-reduced-motion` support (swap springs for short fades), min 4.5:1 text contrast.
- Performance: animate only `transform`/`opacity`/`border-radius`; target steady 60 fps; idle CPU < 1 %, RAM < 150 MB. Pause all animations and polling when collapsed and nothing is playing.
- Light theme variant + "follow system" setting.

Build a small design system first (`ui/`): `Surface`, `Chip`, `IconButton`, `Segmented`, `ProgressRing`, `Donut`, `Toast`, `Tooltip`, with a tokens file.

## 6. IPC contract (Rust ⇄ frontend)

Typed wrappers in `src/lib/ipc.ts`. Commands (examples): `get_taskbar_slot`, `set_click_through(bool)`, `set_focusable(bool)`, `shelf_add(paths|text|image)`, `shelf_list`, `shelf_remove(id)`, `shelf_copy(id)`, `shelf_start_drag(ids)`, `dictionary_lookup(word)`, `usage_summary(range)`, `usage_hourly(date)`, `media_command(cmd)`, `settings_get/set`. Events from Rust: `island://wake`, `island://hotkey/<name>`, `shelf://item-added`, `download://detected`, `media://changed`, `usage://tick`, `display://changed`.

## 7. Settings (JSON in `%APPDATA%/Isle/settings.json`, UI inside expanded panel)

Position mode, monitor, pill width, hover-to-expand on/off, hotkeys (rebindable), theme, shelf expiry, watch Downloads on/off + folder, copy-into-cache on/off, screen-time idle timeout, excluded apps, track window titles (off), launch at startup (`tauri-plugin-autostart`), reduced motion.

## 8. Milestones with acceptance criteria

**M0 — Scaffold (½ day)**
Tauri 2 + React + TS + Tailwind + Framer Motion + Zustand. Transparent, borderless window renders a static pill.
✅ `npm run tauri dev` shows a pill with transparent background and no taskbar entry.

**M1 — Placement & window behavior**
Taskbar free-space detection (UIA), anchor + reposition on taskbar changes, always-on-top keeper, click-through toggling, fullscreen auto-hide, DPI + multi-monitor handling, top-center fallback.
✅ Pill sits in free taskbar space at 100%/125%/150% scaling, stays above the taskbar after clicking the taskbar and Start, never steals focus, hides in fullscreen video/games, clicks pass through transparent area.

**M2 — Island shell & animation**
State machine, hover/click/Esc/hotkey, morph animation, tab bar, design system primitives, settings stub.
✅ Smooth 60 fps morph idle ↔ peek ↔ expanded with no flicker; reduced-motion honored.

**M3 — Shelf (flagship)**
Drop files in, clipboard capture, downloads watcher, SQLite store, thumbnails, click-to-copy as CF_HDROP, native drag-out, context menu, expiry.
✅ Manual test: download a PDF in Chrome → appears in peek → add to shelf → open WhatsApp Web, drag it from the island into the chat and it attaches; also Ctrl+V pastes it. Same for an image and for text.

**M4 — Screen time**
Foreground tracker, idle/lock/sleep handling, SQLite aggregates, icon extraction, donut + hourly chart UI, Today/7-day toggle.
✅ After 10 minutes switching between 3 apps, the numbers match a stopwatch within ±5 s; no time counted while idle > 2 min or locked.

**M5 — Dictionary**
Hotkey focus-input flow, API client + cache, selected-text lookup, saved words, offline behavior.
✅ Lookup of "ephemeral" renders in < 500 ms (cached < 50 ms); airplane mode still shows cached words and a friendly error for new ones.

**M6 — Now Playing**
GSMTC bridge, waveform in idle pill, controls, album-art accent.
✅ Works with Spotify desktop and YouTube in Chrome.

**M7 — Polish & packaging**
Settings UI, autostart, tray icon (Quit/Pause/Settings), error boundaries, crash-safe DB migrations, installer + portable exe, README with GIFs.
✅ Clean install on a fresh Windows 11 VM works; idle CPU < 1 %, RAM < 150 MB over 1 hour.

## 9. Known risks & mitigations

| Risk | Mitigation |
|---|---|
| Windows 11 taskbar is XAML-islands; UIA output can be incomplete | Cache last good result, fall back to a user-defined offset and top-center mode; make slot a draggable setting |
| Topmost fight with the taskbar | Re-assert `HWND_TOPMOST` on timer + foreground events; test after Start/Win+D/Task View |
| Click-through + hover wake | Rust-side cursor polling decides hit-testing; never rely on webview mouse events while click-through is on |
| Drag from browser doesn't expose file paths | Use Tauri drag-drop events (OS-level paths); for browser download bubble, prefer the Downloads watcher flow |
| Drag-out to WhatsApp requires real `CF_HDROP` + correct drop effects | Use proven OLE drag-source crate; test targets list in M3 acceptance |
| Backdrop blur unavailable on transparent window | Solid dark surface design + optional `window-vibrancy` |
| Privacy of usage/clipboard data | Local only, no telemetry, clear-data + pause controls, exclude password managers' clipboard (respect `ExcludeClipboardContentFromMonitorProcessing` / `CanIncludeInClipboardHistory` formats) |
| Antivirus flags hook-based tracker | Use only documented APIs (no injection, no keyloggers; idle via `GetLastInputInfo` only) |

## 10. Instructions to Codex

1. Read this entire file first, then propose the exact crate/package versions you will use (verify they exist and support Tauri 2) before writing code.
2. Work milestone by milestone; commit after each. After each milestone run the build and fix all warnings/errors, and write a short `MILESTONES.md` log of what was verified and how.
3. Keep Win32 code isolated in `src-tauri/src/*` modules behind small safe Rust APIs; document every `unsafe` block.
4. No placeholder UI: every state (loading, empty, error, offline) must be designed and implemented.
5. Do not add features not listed here without asking; do not skip acceptance criteria.
6. When something in sections 4.1–4.4 cannot be done the described way, implement the stated fallback and note it in `MILESTONES.md` rather than silently changing the approach.