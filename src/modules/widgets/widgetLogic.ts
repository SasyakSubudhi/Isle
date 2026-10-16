// Pure helpers for the widgets. No React or Tauri imports, so node --test can
// load this file directly.

export interface Habit {
  id: string;
  name: string;
  /** Local dates the habit was done, as YYYY-MM-DD. */
  done: string[];
}

export interface Weather {
  place: string;
  temperature: number;
  high: number | null;
  low: number | null;
  code: number;
  is_day: boolean;
  fahrenheit: boolean;
  fetched_at: number;
  query: string;
}

/** Local calendar date as YYYY-MM-DD (not UTC, so late evenings count for today). */
export function localDate(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/** The last `count` local dates ending today, oldest first. */
export function lastDays(today: Date, count: number): string[] {
  const days: string[] = [];
  for (let i = count - 1; i >= 0; i--) {
    const d = new Date(today.getFullYear(), today.getMonth(), today.getDate() - i);
    days.push(localDate(d));
  }
  return days;
}

/** Marks or unmarks a day, keeping history sorted and bounded to what Rust accepts. */
export function toggleDay(habit: Habit, day: string): Habit {
  const set = new Set(habit.done);
  if (set.has(day)) set.delete(day);
  else set.add(day);
  return { ...habit, done: [...set].sort().slice(-400) };
}

/**
 * Consecutive days done, counting back from today. If today isn't done yet the
 * streak still counts from yesterday, so it doesn't look broken each morning.
 */
export function streak(habit: Habit, today: Date): number {
  const done = new Set(habit.done);
  let count = 0;
  let offset = done.has(localDate(today)) ? 0 : 1;
  for (;;) {
    const d = new Date(today.getFullYear(), today.getMonth(), today.getDate() - offset);
    if (!done.has(localDate(d))) return count;
    count++;
    offset++;
  }
}

export function newHabit(name: string, id: string): Habit {
  return { id, name: name.trim().slice(0, 40), done: [] };
}

/** WMO weather codes, as used by open-meteo, in plain words. */
export function weatherWords(code: number): string {
  if (code === 0) return 'Clear';
  if (code <= 2) return 'Partly cloudy';
  if (code === 3) return 'Overcast';
  if (code === 45 || code === 48) return 'Fog';
  if (code >= 51 && code <= 57) return 'Drizzle';
  if ((code >= 61 && code <= 67) || (code >= 80 && code <= 82)) return 'Rain';
  if ((code >= 71 && code <= 77) || code === 85 || code === 86) return 'Snow';
  if (code >= 95) return 'Thunderstorm';
  return 'Unsettled';
}

/** A single glyph for the pill-sized weather line. */
export function weatherGlyph(code: number, isDay: boolean): string {
  const words = weatherWords(code);
  if (words === 'Clear') return isDay ? '☀' : '☾';
  if (words === 'Partly cloudy') return isDay ? '⛅' : '☁';
  if (words === 'Rain' || words === 'Drizzle') return '☂';
  if (words === 'Snow') return '❄';
  if (words === 'Thunderstorm') return '⚡';
  return '☁';
}

export const degrees = (n: number | null) => (n == null ? '–' : `${Math.round(n)}°`);
