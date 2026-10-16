# Milestone evidence

## M0 — accepted (2026-10-15)

- Starting workspace empty; initialized from the approved specification.
- Node 22.20.0, Visual Studio C++ workload, Windows SDK and WebView2 detected.
- Rust 1.99.0 MSVC installed. npm exact dependencies resolved and package-lock generated.
- Frontend TypeScript/production build passed without warnings.
- Native `cargo check --locked` and `cargo clippy --locked -- -D warnings` passed.
- `npm run tauri dev` compiled and launched `target/debug/isle.exe` on Windows.
- `npm audit` reported 8 advisories (6 high, 2 moderate) in pinned Vite/Tailwind
  development dependency trees. Versions retained as explicitly requested;
  dev server is bound to 127.0.0.1. This is not an audit-clean dependency set.
- Native screenshot `docs/evidence/m0-native.png` shows the static 136x36 pill
  and underlying desktop content through the transparent 560x360 canvas.
- Live startup diagnostic: taskbar exclusion succeeded; decorated=false;
  resizable=false; topmost=true; scale=1.25. Taskbar exclusion is explicitly
  re-applied with errors propagated from the Windows taskbar API.
- Independent static visual review: ship; no material M0 visual fixes required.
  This does not establish placement, click-through or later milestone behavior.
- Authored SVG mark generated into Windows icons; unnecessary platform outputs
  removed. No native unsafe blocks exist in M0.

## M1 — implemented; native acceptance incomplete (NOT accepted)

- UI Automation taskbar controls, merged free-gap detection, physical-pixel
  placement, explicit Per-Monitor-V2 manifest, display/settings/DPI listeners,
  out-of-context shell hooks, and periodic recovery implemented.
- Cursor polling controls click-through independently of WebView mouse events.
  Transparent canvas remains fixed; only DPI changes resize its physical extent.
- Taskbar gap changes translate the canvas, fullscreen hides it, auto-hide mode
  suppresses it with the hidden taskbar, and failures expose top-center fallback.
  Last-good cache is limited to matching taskbar/monitor geometry and pill width.
- Native Position menu, monitor selection and Alt-drag custom position persist
  settings without replacing unrelated JSON keys. Error results are propagated.
- User's cooler-design request amended the appearance to Cosmic: violet nebula,
  authored orbital mark, sparse particles, stronger glow and static reduced motion.
  Native capture: `docs/evidence/m1-cosmic-native.png`. Independent visual review:
  ship at the current pill scope; this is not native acceptance or whole-app approval.
- Final `npm run build`, `cargo clippy --locked -- -D warnings` and
  `cargo test --locked` passed: 8 tests, zero failures. Tests cover scale-dependent
  geometry, overlap/empty/full gaps, negative coordinates, fallback, settings
  validation and hidden auto-hide taskbar suppression.
- Final `npm run tauri dev` compiled and launched. Actual UIA placement observed
  on DISPLAY1 at 125%; native style readback confirms tool-window/no-activate.
  First-show diagnostic confirms the foreground window was preserved.
- **Unverified:** native 100%/150% scaling, click routing, native Position menu and
  Alt-drag input, repeated Start/Task View interactions, fullscreen video/games,
  Explorer restart, actual auto-hide changes and monitor hotplug/DPI transitions.
  Computer Use can capture the window but refuses input to it as an unusable app
  window. No activation protection was removed to bypass the test limitation.
- Required remaining scenarios are in `docs/M1_ACCEPTANCE.md`.
- M1 remains uncommitted because the plan permits a milestone commit only after
  its native gate passes. M0 commit is `c9d189e`.
- At this stage M2–M7 were unimplemented. The later implementation below supersedes
  that scope status; M1's unverified checks remain open.

### Motion refinement requested by user (2026-10-15)

- Added a gentle 3.8-second planet bob, occasional short comet streak, and
  Framer Motion spring feedback on emblem hover/focus and inner-content press.
- Outer pill geometry remains fixed at 148x36; animated content stays within the
  existing native hit region. No expanded-shell or later module work was added.
- CSS loops pause on document/native visibility changes. Live reduced-motion
  preference updates use a React external-store subscription; CSS also removes
  spatial transforms immediately in reduced-motion mode.
- `npm run build` passed without errors/warnings. Rendered browser checks observed
  running planet/comet/orbit animation, spring focus response, fixed 148x36 bounds,
  no animations/transforms under emulated reduced motion, and resumed motion after
  restoring the browser preference. The temporary emulation and test tabs were removed.
- Native dev server receives the frontend update through Vite HMR. These visual
  checks do not clear M1's remaining native acceptance scenarios or establish
  measured CPU/RAM/frame-rate targets.

## Functionality implementation — 2026-10-15

The user explicitly requested “IMPLEMENT THIS NOW” after asking where the
functionality was. Continued implementation beyond the unavailable interaction
gate, without reclassifying that gate as passed. Changes remain uncommitted under
the original rule that milestone commits require accepted gates.

- **M2:** explicit island reducer, six tool tabs plus Settings, continuous spring
  shell, keyboard navigation, real IPC bootstrap, native cursor hit-region updates,
  reduced motion, loading/error/empty states and presentation privacy.
- **M3:** SQLite Shelf, native clipboard file/text/image formats, OLE drag adapter,
  opt-in background capture/privacy formats/self-write suppression, file references
  and optional cache copies, thumbnails, pins/expiry, Downloads proposals, named
  stacks, cleanup previews retaining originals, autosaved notes and bounded snipping
  capture. Expiry never deletes original files.
- **M4:** foreground hooks, idle/lock/sleep exclusion, local-day/week aggregation,
  SVG charts, exclusions, daily limits with snooze and focus actions.
- **M5:** dictionary requests, cached definitions, saved vocabulary, pronunciation
  and separate selected-text action with clipboard format preservation safeguards.
- **M6:** GSMTC sessions/controls/timeline/artwork, configured Chrome profile launch,
  persisted Pomodoro deadlines and manual transitions, cosmic idle motion and
  explicit presentation mode.
- **M7 implementation:** complete feature settings, tray, autostart, schema version
  migration and README. Release packaging is being verified separately below.

### Verified

- Frontend type check and production build pass, plus 3 reducer workflow tests.
- Rust: 20 tests pass; warnings-as-errors Clippy passes. Tests cover placement,
  settings, cleanup, migration repeat/future rejection, file-reference safety,
  bounded cached-file deletion, pins/expiry, notes/stacks, offline dictionary cache,
  usage aggregation/exclusions and timer recovery/fourth-session long break.
- Actual Windows dev app launches, restores native bootstrap state and displays
  the expanded Shelf with the tool navigation. No sample data is substituted.
- Source review caught and corrected native unit-result/null handling, download
  peek selection, stale dictionary save response and reduced-motion tab movement.

### Unverified and known environment limitations

- Computer Use captures Isle but refuses input as “window is not a usable app
  window”; native transfer to Gmail/WhatsApp/Explorer, clipboard flows, hotkeys,
  selected text, media controls and Chrome-profile launch are not end-to-end verified.
- Ctrl+Alt+S and Ctrl+Alt+Space were reported already registered by Windows in this
  environment. The errors appear in Settings; users can rebind. No external shortcut
  was taken over. Other default shortcuts registered without an error.
- M1's real 100%/150% DPI, Explorer restart, auto-hide, full-screen and monitor
  transition checks remain open. M4's three-app stopwatch accuracy test, network
  latency targets, Spotify/YouTube checks and timer sleep/restart UI checks remain
  unverified beyond the focused automated tests.
- Fresh Windows 11 VM installation, one-hour complete-process-tree performance
  measurement and README GIF capture remain unverified/unavailable. Do not call
  these milestone gates accepted or claim <1% CPU / <150 MB RAM as measured.
- Codex usage and grammar checking remain deferred by the approved release scope.

### Packaging verification

- Final `npm run tauri build` passed with optimized native compilation and NSIS
  packaging. The release includes the reviewed action-result, download selection,
  dictionary response and reduced-motion corrections.
- Copied portable executable and installer to `release/`; SHA-256 values are
  recorded in `release/README.md`. Installer size is approximately 3.09 MiB.
- Launched `release/Isle.exe` on this Windows machine, with `ISLE_OPEN_ON_START=1`
  for inspection. The actual packaged WebView at `http://tauri.localhost/` loaded
  the native bootstrap and expanded Shelf, tool tabs and service footer.
  Screenshot: `docs/evidence/release-expanded.jpg`.
- The four source-review corrections passed their scoped second review. This is
  not end-to-end native interaction approval. The executable is running for the user.
- No new milestone commit was made because native acceptance gates remain open.

## User-recorded glitch corrections — 2026-10-15

Inspected the supplied screenshot and keyframes from the 19.43-second recording.
The user's evidence supersedes the earlier limited visual approval.

- Replaced translucent surfaces with opaque Cosmic dark/light backgrounds to
  prevent underlying desktop text from showing through. Clipped shell content
  during transitions and confined the expanded sky to the header. Stars retain
  their 148×36 source geometry instead of stretching to panel dimensions.
- Download proposals and other Shelf rows cannot shrink into each other. A pending
  download uses compact contextual empty copy; ordinary empty states retain an
  intrinsic minimum and remain scrollable. Header/footer remain separate.
- Removed the native left-button proximity event as a navigation trigger. Only
  actual supported file-drag events can select Shelf. Tab changes preserve pinning.
  Mouse-button proximity also no longer makes transparent space outside the visible
  island intercept clicks; explicit native drag operations retain their capture.
- Added an OLE CF_HDROP target on the outer HWND, replacing its pre-existing target
  without changing WebView child targets. It accepts real file paths only inside
  the reported visible hit region and releases the storage medium. Startup caught
  an already-registered target, corrected by revoking that outer registration before
  installation. The corrected app launched successfully.
- Native capture `docs/evidence/glitches/fixed-native.jpg` confirms opaque content
  and fixed star geometry. An isolated browser fixture with a synthetic download
  rendered the actual components: proposal bottom 598.45, empty copy 611.45–635.4,
  body bottom 655.2 and footer top 663.2. No collision or clipped empty copy.
  Dictionary tab navigation kept Dictionary selected and the panel pinned.
- Four reducer tests and 20 Rust tests pass; frontend build and warnings-as-errors
  Clippy pass. Independent scoped review passed these four UI/input corrections.
- **Still open:** the original Chrome Downloads-to-Shelf drag failure. The native
  adapter compiles and registers, but automation cannot exercise the tool window's
  file-drop interaction. Do not describe Chrome transfer as verified fixed.
- The temporary Downloads layout-check file was removed; source files, copied
  content, personal downloads and app settings were not removed or reset.

### Corrected release delivery — 2026-10-16

- The final optimized build and NSIS packaging completed successfully, including
  the related transparent-area click-through correction. Replaced both files in
  `release/` and refreshed their SHA-256 hashes in `release/README.md`.
- Restarted the packaged app in its normal idle mode, without the QA startup-open
  flag. Chrome-to-Shelf transfer remains pending a real native retry; the four
  reviewed visual/navigation corrections are not whole-app acceptance.

## Native file-drop routing fix — 2026-10-16

- The next user recording showed a PDF reaching expanded Shelf with a prohibited
  cursor and no drag overlay. The previous outer-HWND-only adapter was insufficient.
- Unified OLE registration across the outer window and WebView child windows,
  replacing their separate targets with the same bounded CF_HDROP adapter.
  Registration runs on the UI thread and discovers late-created child windows
  once per second; unchanged HWNDs are not re-registered.
- Added a bounded local `drop-diagnostics.log` in `%LOCALAPPDATA%/Isle`. It records
  target classes, event/format results and item counts, never filenames or content.
- **Actual retry confirmed:** the user reported “The file was added.” Diagnostics
  independently show `enter ... files=1`, `hover ... accepted=true`,
  `drop ... files=1 accepted=true`, then `shelf added=1`. The receiving HWND was
  the registered `Chrome_RenderWidgetHostHWND` child, establishing why handling
  only the outer window did not cover the interaction.
- Four frontend tests and 20 Rust tests pass; production frontend build and
  warnings-as-errors Clippy pass. Version advanced to 0.1.1 with synchronized
  npm/Cargo lockfile metadata; pinned dependency versions are unchanged.
- This verifies the user's single-file input retry. Other transfer formats,
  multi-file inputs and drag-out destinations retain their separate unverified
  acceptance checks. No native gate is implicitly accepted as a whole.
- Optimized 0.1.1 executable and NSIS installer built successfully. Copied the
  executable and `Isle_0.1.1_x64-setup.exe` into `release/`, refreshed SHA-256
  values in the release README, and restarted the packaged app in idle mode.
