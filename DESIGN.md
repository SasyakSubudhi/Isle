---
name: Isle
description: A quiet near-black capsule for the Windows 11 desktop; static M0 implementation.
colors:
  surface: "rgba(10, 10, 12, .97)"
  text: "#f5f5f7"
  muted: "#aaaab5"
  accent: "#a5c7ff"
  highlight: "#ffffff14"
  shadow: "#00000035"
  selection: "#a5c7ff40"
  selection-text: "#fff"
  canvas: "transparent"
typography:
  label:
    fontFamily: "'Segoe UI Variable', 'Segoe UI', sans-serif"
    fontSize: "13px"
    fontWeight: 500
    letterSpacing: ".03em"
rounded:
  capsule: "999px"
  dot: "50%"
spacing:
  canvas-bottom: "12px"
  pill-gap: "12px"
  pill-inline: "17px"
components:
  island-pill:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    typography: "{typography.label}"
    rounded: "{rounded.capsule}"
    padding: "0 17px"
    width: "136px"
    height: "36px"
  ready-dot:
    backgroundColor: "{colors.accent}"
    rounded: "{rounded.dot}"
    width: "4px"
    height: "4px"
---

# Design System: Isle

## Overview

The current M0 visual system is a static, near-black capsule in a transparent native Tauri window, rendered with HTML and CSS for the Windows 11 desktop. It is compact and quiet: a small wave mark, lowercase name and readiness dot provide the entire visible composition.

The user's capsule silhouette and soft highlight/shadow are visual authority. Continuous spring morphing and fluid glow are intended later behavior, not implemented M0 motion. No creative metaphor or broader component language has been established beyond this source-backed capsule.

**Key Characteristics:**
- Near-black capsule on a transparent canvas.
- Pale blue mark and readiness dot with light neutral text.
- Compact system typography and restrained soft depth.
- Static M0 presentation.

## Colors

The palette uses one cool accent against a near-black surface. Frontmatter preserves the source color formats and owns their values.

### Primary
- **Pale Blue Accent:** Used by the inline wave icon, readiness dot and global focus outline.

### Neutral
- **Near-Black Surface:** The capsule fill is slightly transparent.
- **Light Text:** The lowercase wordmark inherits the root text color.
- **Muted Text:** Declared in the source token sheet; currently unused by the M0 markup.
- **Soft Highlight / Soft Shadow:** Translucent white and black supply the capsule's inner highlight and ambient shadow.
- **Transparent Canvas:** The page, root and native window expose the desktop around the capsule.
- **Selection / Selection Text:** Global selection colors exist, although body selection is disabled.

## Typography

The system stack uses Segoe UI Variable, falling back to Segoe UI and sans-serif. Font synthesis is disabled; text rendering uses `optimizeLegibility` and antialiasing.

Only the label role exists in M0: medium-weight lowercase text with slight tracking. Its normative size, weight and spacing appear in frontmatter. There is no implemented heading or body type scale, and line height is not explicitly set.

## Layout

The native window is configured at (560 × 360) logical units, transparent, undecorated and non-resizable. The HTML canvas fills it and uses flex layout to center the capsule horizontally at the bottom, with the frontmatter bottom inset. This is the current window composition; it does not establish final taskbar placement behavior.

The capsule is a fixed-size flex row with centered items, the frontmatter gap and inline padding. Its icon is (19 × 19px). The readiness dot uses `margin-left: auto`. Global `box-sizing: border-box` keeps the capsule padding inside its declared width. The page has no margins and hides overflow. No responsive breakpoints or alternate panel layout are implemented.

## Elevation & Depth

M0 uses a subtle inner top highlight and a soft external shadow. These belong to the capsule itself; the native window shadow is disabled.

### Shadow Vocabulary
- **Capsule highlight and ambient shadow** (`box-shadow: inset 0 1px 0 #ffffff14, 0 6px 14px #00000035`): The complete implemented depth treatment.

## Shapes

The capsule has fully rounded ends; the readiness indicator is circular. Their radii are normative in frontmatter. No visible border is declared. The user's capsule and soft-shadow requirements remain authoritative; generic sharp-corner or flat-surface rules do not apply.

## Components

### Static readiness capsule

The capsule contains an inline SVG wave mark, the text `isle` and a readiness dot. The SVG uses a (24 × 24) viewBox, rounded stroke caps, a main stroke width of (1.8) and a secondary horizontal stroke width of (1.5) at opacity (.45).

The wrapper has `role="img"` and the accessible label `Isle is ready`; the SVG is hidden from accessibility APIs. This is a static presentation, with no click handler, keyboard action, hover state, active state or animation. Do not document it as a button or imply that it opens a panel.

The global focus-visible rule is `outline: 2px solid var(--accent); outline-offset: 4px`. It is available for future focusable elements; the M0 capsule is not focusable. No buttons, inputs, navigation, cards or expanded panels currently exist.

## Do's and Don'ts

### Do:
- Do preserve the capsule silhouette, transparent surroundings and soft highlight/shadow.
- Do use the source-backed tokens and fixed M0 composition when reproducing the current shell.
- Do distinguish declared but unused tokens from visible M0 styling.

### Don't:
- Don't describe continuous spring morphing or fluid glow as implemented M0 behavior.
- Don't invent interactive states, expanded panels or component variants for the static readiness capsule.
- Don't treat the transparent native canvas as an opaque application background.
