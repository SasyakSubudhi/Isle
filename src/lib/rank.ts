/**
 * Scores how well `query` matches `text`: every query character must appear
 * in order. Word starts and consecutive runs score higher. -1 means no match.
 */
export function score(query: string, text: string): number {
  const q = query.trim().toLowerCase();
  if (!q) return 0;
  const t = text.toLowerCase();
  if (t.startsWith(q)) return 1000 - t.length;
  let points = 0;
  let from = 0;
  let previous = -2;
  for (const ch of q) {
    if (ch === ' ') continue;
    const at = t.indexOf(ch, from);
    if (at < 0) return -1;
    const wordStart = at === 0 || /[\s\-_./]/.test(t[at - 1]);
    points += (wordStart ? 10 : 1) + (at === previous + 1 ? 5 : 0);
    previous = at;
    from = at + 1;
  }
  return points;
}

/** Items matching `query`, best first; an empty query keeps the original order. */
export function rank<T>(items: T[], query: string, text: (item: T) => string, limit = 40): T[] {
  if (!query.trim()) return items.slice(0, limit);
  return items
    .map((item, i) => ({ item, i, s: score(query, text(item)) }))
    .filter((x) => x.s >= 0)
    .sort((a, b) => b.s - a.s || a.i - b.i)
    .slice(0, limit)
    .map((x) => x.item);
}
