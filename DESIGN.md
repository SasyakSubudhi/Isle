---
name: Isle
description: Minimal monochrome desktop capsule and compact Operate panel
colors:
  surface: "#0d0d0f"
  text: "#ececee"
  muted: "#8e8e95"
  accent: "#f4f4f5"
  signal: "#8ab4ff"
  selection: "#8ab4ff40"
  selection-text: "#fff"
  canvas: "transparent"
  control: "#ffffff0a"
  control-hover: "#ffffff14"
  line: "#ffffff12"
  primary-ink: "#0d0d0f"
  danger: "#ff9e9e"
  toast: "#1c1c1f"
  error-surface: "#2a1719"
  error-text: "#ffd9d9"
  light-surface: "#fafafa"
  light-text: "#18181b"
  light-muted: "#6b6b73"
  light-accent: "#18181b"
  light-signal: "#2f6fde"
  light-control: "#0000000a"
  light-control-hover: "#00000012"
  light-line: "#00000014"
  light-danger: "#a62142"
typography:
  label:
    fontFamily: "'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "13px"
    lineHeight: 1
    fontWeight: 600
    letterSpacing: "-.02em"
  body:
    fontFamily: "'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "13px"
    lineHeight: 1.45
  title:
    fontFamily: "'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "15px"
    fontWeight: 650
    letterSpacing: "-.03em"
rounded:
  capsule: "999px"
  orbit: "50%"
  panel: "28px"
  control: "9px"
  navigation: "13px"
  tile: "12px"
spacing:
  canvas-bottom: "12px"
  pill-gap: "11px"
  pill-inline: "15px"
  panel-gap: "8px"
  control-gap: "6px"
  grid-gap: "6px"
components:
  island-pill:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    typography: "{typography.label}"
    rounded: "{rounded.capsule}"
    padding: "0 15px"
    width: "148px"
    height: "36px"
  ready-dot:
    backgroundColor: "{colors.ready}"
    rounded: "{rounded.orbit}"
    width: "3px"
    height: "3px"
  operate-panel:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.panel}"
    padding: "15px 16px 10px"
    width: "432px"
    height: "324px"
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.primary-ink}"
    rounded: "{rounded.control}"
    padding: "5px 10px"
  button-secondary:
    backgroundColor: "{colors.control}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "5px 10px"
  input:
    backgroundColor: "{colors.control}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "7px 9px"
  tool-navigation:
    backgroundColor: "{colors.control}"
    rounded: "{rounded.navigation}"
    padding: "3px"
  shelf-tile:
    backgroundColor: "{colors.control}"
    rounded: "{rounded.tile}"
  empty-state:
    textColor: "{colors.muted}"
    padding: "10px 25px"
---

# Design System: Isle

## Overview

**Creative North Star: "Minimal" (2026-10-16, supersedes Cosmic)**

The user asked for a minimal idle pill with many small, quiet animations. The capsule is near-black (light: near-white) with a 1px hairline edge instead of glow. Idle content spans the pill: the status dot sits at the left edge, the lowercase wordmark in the middle and three tide dots at the right. Ambient motion is CSS-only and spans the whole pill: three layered filled waves flow edge to edge at different speeds (7.5 s, 11 s and a reversed 16 s loop), six 2px motes drift and fade, three tide dots on the right rise in sequence, a ripple ring expands from the dot every 6.5 s, the wordmark letters lift 1.5px in sequence every 8 s, a faint sheen crosses the pill about every 11 s and a hairline edge light drifts along the top. Waves and motes are idle-only and hidden in presentation mode. Hover brightens the edge and wordmark, widens the dot into a short bar and draws a 10px hint line. State changes only the rhythm: the halo breathes faster for timer, media and notices, and notices tint the dot with the single signal blue.

Expanded mode keeps the same monochrome surface. White is the primary action color; signal blue (#8ab4ff, light #2f6fde) is reserved for progress, focus rings and completed cycles. Tab changes fade up 6px, list items and settings groups stagger in, tiles lift 1px on hover, buttons press to 96%, toasts spring in and timeline bars grow from the baseline. All loops pause when hidden or when reduced motion is on, and entrances are skipped under `prefers-reduced-motion`.

**Key Characteristics:**
- Calm monochrome capsule with a hairline edge; one signal color for active state.
- `IsleMark` (dot plus breathing halo) replaces the Cosmic emblem; `Ambient` replaces the sky.
- Many small, slow motions; nothing loops faster than 1.1 s except the notice halo.
- Live reduced-motion support and paused animation while hidden.

The Cosmic sections below are historical and no longer describe the current source.

## Colors

The single lavender accent sits against a violet-black surface. Frontmatter owns reusable palette values; artwork gradient details are retained in the sidecar's complete component snippet.

### Primary
- **Lavender Accent:** The global focus outline and source accent token.
- **Luminous Stars:** Fill and stroke color for the decorative star field.
- **Ready Light:** The center of the readiness signal.

Light mode is selectable or follows Windows. It overrides surface, text, muted, accent, control, hover and line using the light tokens in frontmatter; dark Cosmic remains the incumbent identity. Muted text is now actively used across panels. Status colors distinguish destructive actions and errors without changing the lavender action hierarchy.

### Neutral
- **Violet-Black Surface:** Opaque capsule and panel fill, preventing desktop text from bleeding through; the surrounding native canvas remains transparent.
- **Pale Lavender Text:** Inherited foreground for the lowercase wordmark.
- **Muted Lavender:** Secondary labels, helper text and metadata throughout the expanded tools.
- **Transparent Canvas:** Page, root and native window surroundings.
- **Selection / Selection Text:** Declared globally; the body disables text selection.

The emblem's core uses a radial gradient from `#fff5ff` through `#d9b8ff` and `#9264ed` to `#382257`, at offsets (0, .38, .76, 1). Its orbit uses `#8361c7` at opacity (.15), `#c6a0ff` at (.45), `#f7e7ff` at (.7), and `#8f6bd4` at opacity (.2) at the end. Its small cross and dot use `#e7cfff` and `#e5ccff`.

## Typography

The system stack is Segoe UI Variable, Segoe UI, sans-serif. Font synthesis is disabled, with optimized legibility and antialiasing. The compact semibold lowercase wordmark continues in idle mode. Expanded mode uses the body and title roles in frontmatter. Feature headings use weight (600) and tracking (-.025em): h2 (22px), h3 (13px), h4 (12px). Inputs are (12px); toolbar/empty copy commonly (11px); secondary metadata (10px), file metadata (9px). Focus digits are (29px), weight (500), tracking (-.04em), with tabular numerals. Media titles are (19px), line height (1.25). These are observed roles, not a newly imposed scale.

The wordmark sits above the sky at z-index (1), with `text-shadow: 0 1px 5px #100621`.

## Layout

The native frame remains (560 × 360) logical units. The canvas fills it and the shell is absolutely positioned from the native anchor or a browser fallback anchor. Idle width uses the native slot override when present; expanded and peek dimensions are fixed by their respective states. The frontmatter capsule width is the default.

The outer capsule is an isolated flex row with centered items and a native hit region reported as layout changes. An inner content row fills its available width and height with an (11px) gap; only this content row receives press scaling. Emblem and float wrappers are each (23 × 23px). Its emblem is (23 × 23px), nonshrinking. The readiness wrapper is (12 × 12px), uses grid centering and `margin-left: auto`; its center dot is (3 × 3px). Border-box sizing includes padding in the declared width. Page margins are zero and overflow is hidden. No responsive width breakpoints are present. Expanded mode is (432 × 324px), peek mode (292 × 48px), and idle mode defaults to (148 × 36px) with native width override. Shell coordinates are clamped inside the canvas with horizontal margin (8px) and vertical margin (6px). Expansion chooses upward/downward placement around the anchor according to its vertical position; hit bounds are reported through layout animation.

The sky is absolutely inset, clipped to the inherited capsule radius and ignores pointer events. Two elliptical nebulae are (130 × 52px): the near one sits at left (-25px), top (12px), the far one at right (-45px), top (-31px). A faint orbit trail sits at left (28%), top (-27px), measures (135 × 62px), and rotates (-16deg).

## Elevation & Depth

The capsule combines inner edge highlights, a soft black shadow and violet glow. Its translucent overlay is `linear-gradient(115deg,#e9d6ff0d,transparent 35%,transparent 65%,#ad80f20a)`. The authored emblem has `filter: drop-shadow(0 0 5px #c9a1ff55)`.

### Shadow Vocabulary
- **Capsule depth:** `inset 0 1px 0 #f0dcff26, inset 0 -1px 0 #9e71ee14, 0 6px 14px #0000004d, 0 0 22px #864ddd29`.
- **Ready light glow:** `0 0 6px #c699ff99`.
- **Wordmark separation:** `0 1px 5px #100621` as a text shadow.

The comet is a (28 × 1px) streak at left (0), top (10px), using `linear-gradient(90deg,transparent,#cba4ff80,#fff0ff)` with `box-shadow: 1px 0 4px #e6c2ff66`.

The near nebula uses `radial-gradient(ellipse,#a36aef70 0%,#7c3fbb2e 38%,transparent 70%)`; the far nebula uses `radial-gradient(ellipse,#e6adff60 0%,#9864ff35 36%,transparent 70%)`.

## Shapes

The capsule retains fully rounded ends. Nebulae, readiness orbit and trail use elliptical or circular rounding. There is no capsule border; the trail uses `1px solid #d5b4ff1a`. The readiness orbit uses `1px solid #ceb0ff29` with top edge `#dfc5ff99`. Preserve the user's capsule and soft depth requirements.

## Components

### Operate panel and module surfaces

The expanded panel uses a (28px) corner radius and padding from frontmatter. A vertical content flex layout has an (8px) gap: (28px) header, six icon tool tabs, flexible clipped body with scrollable feature content, and a footer separated by a one-pixel tonal rule and (7px) top padding. The shell clips all content with `overflow: hidden`. The expanded sky fades to opacity (.32) and is limited to a (58px) header band with `bottom: auto`. Its star SVG remains (148 × 36px), right-aligned at (12px), with `left: auto`, instead of stretching across the expanded panel. Header controls expose Settings, pin and collapse. Changing tool tabs preserves the existing pin value unless a caller explicitly supplies one. Normal pointer movement does not select Shelf; actual file drag/drop can activate Shelf.

The six tabs are Shelf, Screen time, Dictionary, Focus timer, Now Playing and Scratchpad. Settings is a separate header destination with sticky Save changes, grouped labels, native checkboxes, profile setup help and destructive confirmation. Footer shortcuts are Gmail, Outlook and WhatsApp; choosing one without a Chrome profile routes to Settings and explains setup. Their dark-theme icons are pink, blue and green; light-theme icons use the accent.

Shelf children do not shrink. The standard Shelf empty state has `flex: 1 0 110px`, minimum height (110px), and padding (5px 20px). When a download proposal is pending or clipboard history is disabled, a compact paragraph replaces the large empty-state illustration. It uses (11px) muted centered text with padding (4px 16px), keeping the proposal and next-step copy separate. Shelf uses a two-column grid with (6px) gap and tiles at minimum (57px) height, including thumbnail/icon, ellipsized name, metadata and actions. Detail and cleanup views keep long paths/text selectable and wrapped. Dictionary uses a compact search field, definition hierarchy, pronunciation/save actions and cached-state labels. Focus uses a (132px) ring with tabular timer digits, cycle dots and timer actions. Media uses (92 × 92px) artwork with (16px) corners, track information, waveform, seek range and playback controls. Scratchpad is a flexible borderless textarea with line height (1.7) and local save status. Screen time uses a (118px) donut, app legend and timeline; charts reflect API results, not decorative sample values.

The source consumes backend state; empty and unavailable states explain the next action rather than showing mock content. Shelf, dictionary, notes, usage, media and bootstrap errors share the centered orbit/icon, heading and muted explanation vocabulary. Toasts are (11px), radius (11px), bottom offset (39px); normal toasts time out after (2400ms), errors after (6500ms). Error alerts use the dedicated error surface/text. Drag state overlays a dashed accent border and the message "Drop it here" with "Files stay where they are." Clipboard permission and download proposals use compact explanatory strips.

Presentation mode hides the sky, private previews, notifications and normal tools; it shows a privacy explanation, exit action and active-timer remainder when present. No fabricated content should fill an empty or private state.

### Controls and navigation

Standard buttons use the frontmatter padding and radius, minimum height (27px), gap (6px), and a background transition (.12s ease). Hover uses the control-hover color; disabled controls use opacity (.4). Primary actions use accent fill with dark ink, weight (600), and hover brightness (1.06). Light-theme primary actions use white text. Icon buttons are (27 × 27px), transparent at rest; active state uses accent foreground and control-hover fill. Text and compact variants retain their observed smaller sizing.

Text inputs, selects and textareas use a one-pixel tonal border and selectable text. Placeholders use muted color at opacity (.85); caret uses accent. Checkboxes are (16 × 16px) with native accent color. Inputs and panel buttons use a (2px) accent focus outline with (2px) offset. The six tool buttons share their container equally and are (31px) high, with a rounded active highlight and `aria-pressed` state. Small segmented controls use (23px) minimum-height buttons.

### Cosmic capsule

The capsule composes CosmicSky, CosmicMark, the `isle` label and readiness signal. It has `role="button"`, `tabIndex={0}`, an Open Isle accessible label in collapsed mode. Click or Enter/Space opens the tool panel; right-click opens native position settings while collapsed. Escape collapses the shell. Expanded mode uses a named region and wraps keyboard Tab focus within visible controls. Alt plus primary pointer-down starts native custom drag. Native position-menu and custom-drag calls are gated on native availability; panel state can also render outside the native runtime. Optional hover expansion waits (150ms); pointer exit schedules collapse after (600ms) unless pin, drag, capture or focused editing keeps the panel open. Pointer enter/leave sets the engagement state for the emblem spring; press feedback scales only the inner content. Inner press feedback does not independently scale the hit region; the shell itself now changes dimensions through the layout spring and reports updated bounds. Native OLE targets cover the outer window and WebView child windows, including late-created children. The user confirmed a successful PDF input retry, also recorded in bounded local diagnostics. This does not establish other native transfers or whole-app acceptance.

Keyboard focus uses `outline: 2px solid var(--accent); outline-offset: 4px`. Decorative SVG and sky are hidden from accessibility APIs; the readiness wrapper is also hidden. The sky and overlay ignore pointer events. Panel controls use the tighter focus offset described above. Normal idle mode can show a timer ring, playing-media waveform/artwork, or readiness signal; peek mode shows real event text and an optional download Add action.

### Local orbital artwork

The authored emblem uses a (32 × 32) SVG viewBox with a center circle at (16,16), radius (7), an inclined orbit path with rounded caps and stroke width (1.3), plus a small cross and star dot. CosmicSky contains six circles and one cross in a (148 × 36) SVG viewBox with `preserveAspectRatio="none"`. It uses local CSS and inline SVG, without remote imagery, video or canvas polling.

### Motion and reduced motion

Near nebula animation is `nebula-flow 9s ease-in-out infinite alternate`; far nebula animation is `nebula-flow 12s ease-in-out -5s infinite alternate-reverse`. They move from `translate3d(-6px,0,0) rotate(-10deg) scale(.9)` to `translate3d(24px,-8px,0) rotate(18deg) scale(1.15)`.

Stars use `star-breathe 5s ease-in-out infinite alternate`, varying opacity from (.3) to (.8); their base opacity is (.65). The readiness orbit uses `orbit-turn 10s linear infinite`, rotating to (360deg). The floating planet uses `planet-float 3.8s ease-in-out infinite`: at (0%/100%) it translates Y by (1px) and rotates (-3deg); at (50%) it translates Y by (-1.5px) and rotates (3deg). The comet uses `comet-pass 6.8s ease-out 1.2s infinite`. It starts at `translate3d(-32px,14px,0) rotate(-12deg)` with opacity (0), reaches opacity (.85) at (5%), then reaches `translate3d(164px,-12px,0) rotate(-12deg)` with opacity (0) at (24%) and stays there through (100%), leaving a long quiet interval. Its initial delay applies before the repeating animation, not between every pass.

The emblem uses Framer Motion with `initial={false}`. Pointer engagement targets `y: -1`, `rotate: -9`, `scale: 1.12`; disengagement returns `y: 0`, `rotate: 0`, `scale: 1`. Its spring uses stiffness (300), damping (18), mass (.65). The inner content rests at scale (1) and presses to (.97), using the idle-content transition duration (.14s); reduced mode uses (.08s). Shell layout uses stiffness (380), damping (30), mass (.9); reduced mode uses duration (.1s). Panel content enters from Y (5px) with delay (.1s), duration (.16s); reduced mode removes translation and delay. The selected-tab highlight uses stiffness (400), damping (32), or duration (0) when reduced. The source uses AnimatePresence for content swaps.

Document visibility and native `island://visibility` events determine the paused state. Hidden state pauses CSS animation on nebulae, stars, readiness orbit, float and comet, returns the emblem target to rest, and disables press scaling. The live `useSyncExternalStore` preference hook subscribes to media-query changes, so reduced motion updates after mount. Under `prefers-reduced-motion: reduce`, all five animated classes set `animation: none` and `will-change: auto`, the comet sets opacity (0), and emblem/content transforms become `none !important`. The emblem targets rest with duration (0). App-level reduced motion also pauses ambient CSS through the data attribute and replaces layout motion with short fades. The decoration remains visible and static, except the comet is hidden.

## Do's and Don'ts

### Do:
- Do preserve Cosmic's violet capsule, local orbital artwork and soft luminous depth.
- Do preserve keyboard focus and the static reduced-motion presentation.
- Do honor native slot positioning and width instead of assuming the fallback composition is always active.

### Don't:
- Don't treat the earlier blue M0 screenshot as the current design.
- Don't describe CSS nebula drift as a fluid simulation or confuse source implementation with completed native acceptance.
- Don't replace real empty, permission or unavailable states with mock records or claim unverified native workflows passed.
