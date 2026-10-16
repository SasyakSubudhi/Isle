/** What Windows reports right now (see src-tauri/src/activity). */
export interface LiveActivity {
  battery: { percent: number; charging: boolean; plugged_in: boolean; saver: boolean } | null;
  downloads: { name: string; bytes: number; bytes_per_second: number }[];
  sensors: { kind: 'camera' | 'microphone' | 'screen'; app: string; since: number | null }[];
}

/** The one thing the pill morphs to show, most important first. */
export type Live =
  | { kind: 'screen' | 'camera' | 'microphone' | 'call'; app: string; since: number | null }
  | { kind: 'download'; name: string; count: number; bytes: number; rate: number }
  | { kind: 'power'; percent: number; pluggedIn: boolean }
  | { kind: 'battery-low'; percent: number; critical: boolean };

export interface LiveInput {
  activity: LiveActivity | null;
  /** When the charger was plugged in or pulled out, for a short-lived notice. */
  powerChangedAt: number | null;
  now: number;
  timerActive: boolean;
  mediaPlaying: boolean;
  privateMode: boolean;
  /** Do Not Disturb: only the call/sensor indicator and a critical battery show. */
  dnd?: boolean;
}

const CALL_APPS = ['Zoom', 'Teams', 'Webex', 'Slack', 'Discord', 'Skype', 'Google Meet'];
export const POWER_NOTICE_MS = 4500;
export const LOW_BATTERY = 20;
export const CRITICAL_BATTERY = 10;

/**
 * Picks the live activity to show. Privacy indicators come first, then work in
 * progress, then a brief charger notice. A low battery waits behind the timer
 * and music unless it is critical. Presentation mode shows nothing.
 */
export function pickLive(input: LiveInput): Live | null {
  const { activity, privateMode } = input;
  if (privateMode || !activity) return null;
  for (const kind of ['screen', 'camera', 'microphone'] as const) {
    const use = activity.sensors.find((s) => s.kind === kind);
    if (use)
      return {
        kind: kind === 'microphone' && CALL_APPS.includes(use.app) ? 'call' : kind,
        app: use.app,
        since: use.since,
      };
  }
  if (input.dnd) {
    const battery = activity.battery;
    return battery && !battery.plugged_in && battery.percent <= CRITICAL_BATTERY
      ? { kind: 'battery-low', percent: battery.percent, critical: true }
      : null;
  }
  if (activity.downloads.length) {
    const [first] = activity.downloads;
    return {
      kind: 'download',
      name: first.name,
      count: activity.downloads.length,
      bytes: activity.downloads.reduce((n, d) => n + d.bytes, 0),
      rate: activity.downloads.reduce((n, d) => n + d.bytes_per_second, 0),
    };
  }
  const battery = activity.battery;
  if (!battery) return null;
  if (input.powerChangedAt !== null && input.now - input.powerChangedAt < POWER_NOTICE_MS)
    return { kind: 'power', percent: battery.percent, pluggedIn: battery.plugged_in };
  const critical = battery.percent <= CRITICAL_BATTERY;
  if (
    !battery.plugged_in &&
    battery.percent <= LOW_BATTERY &&
    (critical || (!input.timerActive && !input.mediaPlaying))
  )
    return { kind: 'battery-low', percent: battery.percent, critical };
  return null;
}

export const formatRate = (bytesPerSecond: number) =>
  bytesPerSecond >= 1_048_576
    ? `${(bytesPerSecond / 1_048_576).toFixed(1)} MB/s`
    : bytesPerSecond >= 1024
      ? `${Math.round(bytesPerSecond / 1024)} KB/s`
      : '';

export const formatElapsed = (since: number | null, now: number) => {
  if (since === null || since > now) return '';
  const seconds = Math.floor((now - since) / 1000);
  const h = Math.floor(seconds / 3600),
    m = Math.floor((seconds % 3600) / 60),
    s = seconds % 60;
  const pad = (n: number) => n.toString().padStart(2, '0');
  return h ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
};
