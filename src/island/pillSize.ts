/** The collapsed pill snaps between three widths so state changes read as one smooth morph. */
export const PILL_MEDIUM = 236;
export const PILL_WIDE = 324;
export const PANEL = { width: 432, height: 324 };

export type PillState = {
  expanded: boolean;
  dropping: boolean;
  dock: boolean;
  peek: boolean;
  live: boolean;
  mediaPlaying: boolean;
};

/** Compact (the taskbar slot), medium (media, live activity, file drop) or wide (peek, dock). */
export function pillSize(s: PillState, compact: number) {
  if (s.expanded) return PANEL;
  if (s.dock) return { width: Math.max(compact, PILL_WIDE), height: 64 };
  if (s.peek) return { width: Math.max(compact, PILL_WIDE), height: 48 };
  if (s.dropping) return { width: Math.max(compact, PILL_MEDIUM), height: 48 };
  if (s.live || s.mediaPlaying) return { width: Math.max(compact, PILL_MEDIUM), height: 36 };
  return { width: compact, height: 36 };
}
