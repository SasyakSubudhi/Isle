// Pure helpers for the context-aware pill. No React or Tauri imports, so
// node --test can load this file directly.

export type ContextKind = 'code' | 'meeting' | 'reading' | 'writing';

/** What the native side reports: the activity, the app's executable name and when it began. */
export interface ContextInfo {
  kind: ContextKind;
  app: string;
  since: number;
}

const labels: Record<ContextKind, string> = {
  code: 'Coding',
  meeting: 'Meeting',
  reading: 'Reading',
  writing: 'Writing',
};

export const contextLabel = (kind: ContextKind) => labels[kind] ?? 'Active';

/** "now" under a minute, then "12m", then "1h 05m". */
export function formatElapsed(ms: number): string {
  const minutes = Math.floor(Math.max(0, ms) / 60_000);
  if (minutes < 1) return 'now';
  if (minutes < 60) return `${minutes}m`;
  return `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, '0')}m`;
}

/** Meetings show a running clock so you can watch the time; the rest update once a minute. */
export function tickMs(kind: ContextKind) {
  return kind === 'meeting' ? 1000 : 15_000;
}

/** mm:ss under an hour, h:mm:ss after. */
export function formatMeeting(ms: number): string {
  const total = Math.floor(Math.max(0, ms) / 1000);
  const h = Math.floor(total / 3600),
    m = Math.floor((total % 3600) / 60),
    s = total % 60;
  const pad = (n: number) => String(n).padStart(2, '0');
  return h ? `${h}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`;
}
