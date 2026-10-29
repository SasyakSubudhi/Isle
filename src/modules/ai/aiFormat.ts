/** 1234 → "1.2K", 3_400_000 → "3.4M". */
export const formatTokens = (n: number) =>
  n >= 1_000_000
    ? `${(n / 1_000_000).toFixed(n >= 10_000_000 ? 0 : 1)}M`
    : n >= 1000
      ? `${(n / 1000).toFixed(n >= 10_000 ? 0 : 1)}K`
      : `${Math.round(n)}`;

/** "in 2h 14m", "in 5m", "in 3d 4h". */
export function until(at: number, now: number) {
  const minutes = Math.max(0, Math.ceil((at - now) / 60_000));
  if (minutes >= 48 * 60)
    return `in ${Math.floor(minutes / 1440)}d ${Math.floor((minutes % 1440) / 60)}h`;
  if (minutes >= 60) return `in ${Math.floor(minutes / 60)}h ${minutes % 60}m`;
  return minutes ? `in ${minutes}m` : 'now';
}

/** "3 min ago", "2 h ago", "4 d ago". */
export function ago(at: number, now: number) {
  const minutes = Math.max(0, Math.floor((now - at) / 60_000));
  if (minutes < 1) return 'just now';
  if (minutes < 60) return `${minutes} min ago`;
  if (minutes < 48 * 60) return `${Math.floor(minutes / 60)} h ago`;
  return `${Math.floor(minutes / 1440)} d ago`;
}

/** Codex window lengths: 300 → "5-hour", 10080 → "Weekly". */
export const windowName = (minutes: number) =>
  minutes === 10080
    ? 'Weekly'
    : minutes % 1440 === 0 && minutes
      ? `${minutes / 1440}-day`
      : minutes % 60 === 0 && minutes
        ? `${minutes / 60}-hour`
        : `${minutes}-minute`;

/** Clock time for a reset, with the weekday when it isn't today. */
export function resetTime(at: number, now: number) {
  const date = new Date(at);
  const time = date.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
  return new Date(now).toDateString() === date.toDateString()
    ? time
    : `${date.toLocaleDateString([], { weekday: 'short' })} ${time}`;
}
