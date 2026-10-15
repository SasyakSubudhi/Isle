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

## M1 — next

Taskbar placement, input behavior, fullscreen handling and DPI/monitor checks
have not yet been implemented or accepted.
