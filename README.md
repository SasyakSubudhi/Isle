# Isle

A Windows 11 taskbar island with a Cosmic violet design, small orbital animations
and a single shell that springs open into your tools. Current verification and
remaining acceptance checks are recorded in [MILESTONES.md](MILESTONES.md).

## Development

Install Node.js, Rust's `x86_64-pc-windows-msvc` toolchain, Visual Studio C++ build
tools, a Windows SDK and WebView2. Then run:

```powershell
npm ci
npm run tauri dev
```

The Tauri launcher also finds Rust in rustup's standard user directory, so a
terminal restart after installation is not required.

## Using Isle

Click or hover over the pill to open its tools. Pin the panel to keep it open;
Escape or the close button collapses it. Right-click the collapsed pill for
position controls, or hold Alt and drag it. The tray menu also opens Isle and
Settings, pauses tracking, controls presentation mode and quits the app.

- **Shelf:** drop files, paste the clipboard, add text/links, or take a screen
  snip. Click a tile to copy; drag file tiles into another app. The actions button
  offers pinning, labels, stacks, cleanup previews and removal. Original files
  are never deleted by Shelf removal or expiry.
- **Clipboard history:** open History and explicitly enable capture. Explicit
  paste and screenshots work with background history off.
- **Screen time:** view real foreground app time today or across seven days.
- **AI usage:** Claude Code tokens in the current five-hour window with its reset time, and the exact Codex (ChatGPT plan) limits, read from those tools' logs on this PC. Nothing is sent anywhere.
  Select an app for its timeline, daily limit or exclusion. Idle, lock and sleep
  are excluded; pause tracking in Settings or the tray.
- **Dictionary:** look up English words, play available pronunciation and save
  words. Previously looked-up definitions work offline; first lookup and audio
  need an internet connection. Selected-text lookup uses its own hotkey.
- **Focus:** start/pause/resume a Pomodoro. Defaults are 25/5/15 minutes, with a
  long break after four focus sessions. Each next session starts explicitly.
  Running timers recover from timestamps after restart or sleep.
- **Now Playing:** Windows-supported media sessions supply artwork, timeline
  and controls. Unsupported players show an unavailable state.
- **Scratchpad:** create, pin and edit notes that autosave locally and do not
  expire with Shelf items.

The Gmail, YouTube and WhatsApp buttons open Chrome. First select your existing
signed-in profile in Settings; account credentials are never stored by Isle.
You can change each shortcut's URL in Settings. Presentation mode hides
previews and notifications until explicitly exited while timers/tracking run.

Defaults: Ctrl+Alt+Space toggles Isle, Ctrl+Alt+D opens Dictionary, Ctrl+Alt+S
opens Shelf, Ctrl+Alt+T opens Screen time, Ctrl+Alt+Shift+D looks up selected text.
Other apps may own these combinations. Settings reports conflicts and allows
rebinding. The pill and tray remain available when a hotkey conflicts.

## Local data and builds

Settings: `%APPDATA%/Isle/settings.json`. SQLite and cached files:
`%LOCALAPPDATA%/Isle`. No telemetry, cloud sync or automatic message sending.
Dictionary requests send the lookup word to dictionaryapi.dev.

```powershell
npm test
npm run build
cd src-tauri
cargo test --locked
cargo clippy --locked -- -D warnings
cd ..
npm run tauri build
```

The portable executable is `src-tauri/target/release/isle.exe`; the installer is
under `src-tauri/target/release/bundle/nsis`. WebView2 remains required. Packaging
does not establish fresh-VM installation or the one-hour CPU/RAM targets.

M0 is accepted and committed. Later native acceptance is incomplete; see
[the checklist](docs/M1_ACCEPTANCE.md) and milestone log. Functionality was
implemented following the user's instruction to proceed, with unverified gates
retained explicitly. Codex usage and grammar checking remain deferred. Exact
pinned development dependencies have npm advisories recorded in the milestone log.
