# M1 native acceptance checklist

M1 is **not accepted**. Passing unit tests is not a substitute for these native
checks. M2 and later must wait until the mandatory checks below pass.

## Current evidence

- Windows native launch and actual UI Automation taskbar placement at 125%:
  observed. `docs/evidence/m1-cosmic-native.png` captures the pill between Widgets
  and Start/Search, with the surrounding canvas transparent.
- Frontend build, Rust warnings-as-errors and coordinate tests: passed.
- Final native style readback confirms tool-window/no-activate. First show logged
  `Isle shown without changing foreground: true`.
- Eight tests cover 100%, 125%, 150%, overlapping/nested controls, full/empty
  UIA output, negative monitor coordinates, fallback and auto-hide suppression.
- Computer Use captured the no-activate window but refused input with
  `verify current window before using captured target: window is not a usable app window`.
  The app's no-activate behavior was not disabled to bypass this limitation.

## Required manual scenarios

1. Run `npm run tauri dev`. Confirm the pill is in a free taskbar gap and no Isle
   taskbar button or Alt-Tab entry appears. Repeat at Windows scaling 100%,125%,150%.
2. Click an app underneath the large transparent area above the pill. The click
   must reach that app. Hover the pill, then click/leave it; it must remain usable
   and return the surrounding canvas to click-through.
3. Click/right-click the pill to open its native Position menu. Switch between
   taskbar/free-space and top-center. Hold Alt while dragging the pill to store a
   custom position. Restart and confirm the chosen position survives.
4. Keep another app focused. The pill's initial appearance, repositioning and
   idle mouse interaction must not steal keyboard focus. Position menu behavior
   is an explicit interaction; verify focus returns after it closes.
5. Click taskbar, open Start and Task View, and return to the desktop. The pill
   must recover above the taskbar without covering shell controls.
6. Enter fullscreen video and a fullscreen game on the selected monitor. Isle
   must hide, then recover when fullscreen ends. Also test borderless fullscreen.
7. Open/close pinned/running apps until gaps change. Confirm placement updates
   after the debounce; when no gap fits, it uses top-center. UIA errors may reuse
   only a last good gap from the same monitor/taskbar geometry and pill width.
8. Toggle taskbar auto-hide: Isle should hide with the taskbar in taskbar mode,
   reappear in a free gap when the taskbar reveals, and leave its reveal edge free.
9. Restart Explorer. Confirm recovery without restarting Isle. This is disruptive
   to the user's desktop, so perform it deliberately in the acceptance session.
10. Attach a second monitor, choose it in the Position menu, move it in display
    layout, change its DPI, and disconnect it. Confirm correct placement and
    fallback to the primary monitor without offscreen stranding.
11. Enable Windows reduced motion: the cosmic sky remains visible but static.

Record Windows build, monitor/DPI, pass/fail, and supporting evidence for each.
Do not mark a scenario passed solely because its implementation compiles.

## Current feature boundary

The pill currently opens native Position controls. Expansion, Shelf/clipboard,
screen time, dictionary, media, timers, Chrome shortcuts and other modules have
not begun. There are no fabricated module screens or sample usage values.
