// Pure helpers for the Settings form. Kept free of React and Tauri imports so
// they can be tested directly with node --test.

const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

/** Keys the user changed in `draft` compared with the settings it started from. */
export function changedFields<T extends object>(base: T, draft: T): Partial<T> {
  const patch: Partial<T> = {};
  for (const key of Object.keys(draft) as (keyof T)[]) {
    if (!same(base[key], draft[key])) patch[key] = draft[key];
  }
  return patch;
}

/**
 * Settings changed elsewhere (tray menu, another button). Adopt the new value
 * for every field the user hasn't edited, and keep their unsaved edits.
 */
export function rebaseDraft<T extends object>(base: T, draft: T, incoming: T): T {
  const edits = changedFields(base, draft);
  return { ...incoming, ...edits };
}

/** Parses a number field and keeps it inside the range Rust accepts. */
export function clampNumber(raw: string, min: number, max: number, fallback: number): number {
  const value = Number(raw);
  if (raw.trim() === '' || !Number.isFinite(value)) return fallback;
  return Math.min(max, Math.max(min, Math.round(value)));
}

export interface KeyLike {
  key: string;
  code?: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

const modifierKeys = new Set(['Control', 'Alt', 'Shift', 'Meta', 'OS', 'AltGraph']);
const named: Record<string, string> = {
  ' ': 'Space',
  ArrowUp: 'Up',
  ArrowDown: 'Down',
  ArrowLeft: 'Left',
  ArrowRight: 'Right',
  Escape: 'Escape',
  Enter: 'Enter',
  Tab: 'Tab',
  Backspace: 'Backspace',
  Delete: 'Delete',
  Insert: 'Insert',
  Home: 'Home',
  End: 'End',
  PageUp: 'PageUp',
  PageDown: 'PageDown',
};

/**
 * Turns a key press into the "Ctrl+Alt+D" form the global-shortcut plugin
 * parses. Returns null until a non-modifier key is pressed with at least one
 * modifier, since a bare letter would hijack normal typing.
 */
export function hotkeyFromEvent(event: KeyLike): string | null {
  if (modifierKeys.has(event.key)) return null;
  const parts: string[] = [];
  if (event.ctrlKey) parts.push('Ctrl');
  if (event.altKey) parts.push('Alt');
  if (event.shiftKey) parts.push('Shift');
  if (event.metaKey) parts.push('Super');
  if (parts.length === 0) return null;
  let key: string | undefined;
  if (event.code?.startsWith('Key')) key = event.code.slice(3);
  else if (event.code?.startsWith('Digit')) key = event.code.slice(5);
  else if (/^F\d{1,2}$/.test(event.key)) key = event.key;
  else key = named[event.key];
  if (!key) return null;
  return [...parts, key].join('+');
}
