// Smart clipboard: works out what a copied piece of text is (link, color,
// code, one-time code, address, email, phone) and which one-tap actions fit.
// Everything here runs on-device and is free of React and Tauri imports so it
// can be tested directly with node --test.

export type ClipKind = 'link' | 'email' | 'phone' | 'color' | 'otp' | 'address' | 'code' | 'text';

export interface Rgba {
  r: number;
  g: number;
  b: number;
  a: number;
}

export interface ClipAction {
  id: string;
  label: string;
  /** copy: put `value` on the clipboard. open: open `value` as a URL. save: add `value` to Shelf. */
  type: 'copy' | 'open' | 'save';
  value: string;
}

export interface ClipInfo {
  kind: ClipKind;
  /** Short label for the tile, such as "Color" or "Code 4821". */
  label: string;
  /** Present when kind is "color", for drawing a swatch. */
  color?: Rgba;
  actions: ClipAction[];
}

const MAX_SCAN = 20_000;

// ---------- colors ----------

const clamp = (n: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, n));
const round = (n: number, places = 0) => {
  const f = 10 ** places;
  return Math.round(n * f) / f;
};

function channel(raw: string, max: number): number | null {
  const value = raw.trim();
  if (value.endsWith('%')) {
    const n = Number(value.slice(0, -1));
    return Number.isFinite(n) ? clamp((n / 100) * max, 0, max) : null;
  }
  const n = Number(value);
  return value !== '' && Number.isFinite(n) ? clamp(n, 0, max) : null;
}

function alpha(raw: string | undefined): number | null {
  if (raw === undefined) return 1;
  return channel(raw, 1);
}

function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  const hue = (((h % 360) + 360) % 360) / 360;
  if (s === 0) return [l * 255, l * 255, l * 255];
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  const part = (t: number) => {
    const x = t < 0 ? t + 1 : t > 1 ? t - 1 : t;
    if (x < 1 / 6) return p + (q - p) * 6 * x;
    if (x < 1 / 2) return q;
    if (x < 2 / 3) return p + (q - p) * (2 / 3 - x) * 6;
    return p;
  };
  return [part(hue + 1 / 3) * 255, part(hue) * 255, part(hue - 1 / 3) * 255];
}

/** Parses #rgb, #rgba, #rrggbb, #rrggbbaa, rgb()/rgba() and hsl()/hsla(). */
export function parseColor(input: string): Rgba | null {
  const text = input.trim().toLowerCase().replace(/;$/, '');
  const hex = /^#?([0-9a-f]{3}|[0-9a-f]{4}|[0-9a-f]{6}|[0-9a-f]{8})$/.exec(text);
  // A bare run is too often a word or a number ("decade", "123456"), so hex
  // without "#" only counts at six or eight characters mixing letters and digits.
  const bare = hex && hex[1].length >= 6 && /[a-f]/.test(hex[1]) && /\d/.test(hex[1]);
  if (hex && (text.startsWith('#') || bare)) {
    let digits = hex[1];
    if (digits.length <= 4) digits = [...digits].map((c) => c + c).join('');
    const n = (i: number) => parseInt(digits.slice(i, i + 2), 16);
    return { r: n(0), g: n(2), b: n(4), a: digits.length === 8 ? round(n(6) / 255, 3) : 1 };
  }
  const fn = /^(rgba?|hsla?)\(\s*([^)]*)\)$/.exec(text);
  if (!fn) return null;
  const parts = fn[2]
    .split(/\s*[,/]\s*|\s+/)
    .map((p) => p.trim())
    .filter(Boolean);
  if (parts.length < 3 || parts.length > 4) return null;
  const a = alpha(parts[3]);
  if (a === null) return null;
  if (fn[1].startsWith('rgb')) {
    const [r, g, b] = parts.slice(0, 3).map((p) => channel(p, 255));
    if (r === null || g === null || b === null) return null;
    return { r: Math.round(r), g: Math.round(g), b: Math.round(b), a: round(a, 3) };
  }
  const h = Number(parts[0].replace(/deg$/, ''));
  const s = channel(parts[1].endsWith('%') ? parts[1] : `${parts[1]}%`, 1);
  const l = channel(parts[2].endsWith('%') ? parts[2] : `${parts[2]}%`, 1);
  if (!Number.isFinite(h) || s === null || l === null) return null;
  const [r, g, b] = hslToRgb(h, s, l);
  return { r: Math.round(r), g: Math.round(g), b: Math.round(b), a: round(a, 3) };
}

const hex2 = (n: number) => Math.round(n).toString(16).padStart(2, '0');

export function toHex({ r, g, b, a }: Rgba): string {
  return `#${hex2(r)}${hex2(g)}${hex2(b)}${a < 1 ? hex2(a * 255) : ''}`.toUpperCase();
}

export function toRgb({ r, g, b, a }: Rgba): string {
  return a < 1 ? `rgba(${r}, ${g}, ${b}, ${a})` : `rgb(${r}, ${g}, ${b})`;
}

export function toHsl({ r, g, b, a }: Rgba): string {
  const [rn, gn, bn] = [r / 255, g / 255, b / 255];
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const l = (max + min) / 2;
  const d = max - min;
  let h = 0;
  let s = 0;
  if (d !== 0) {
    s = d / (1 - Math.abs(2 * l - 1));
    if (max === rn) h = ((gn - bn) / d) % 6;
    else if (max === gn) h = (bn - rn) / d + 2;
    else h = (rn - gn) / d + 4;
    h *= 60;
    if (h < 0) h += 360;
  }
  const body = `${Math.round(h)}, ${Math.round(s * 100)}%, ${Math.round(l * 100)}%`;
  return a < 1 ? `hsla(${body}, ${a})` : `hsl(${body})`;
}

// ---------- links, emails, phones ----------

const TRACKING = ['fbclid', 'gclid', 'dclid', 'msclkid', 'mc_cid', 'mc_eid'];

/** Removes utm_* and click-id parameters, matching the Rust "Clean link" action. */
export function cleanLink(url: string): string {
  try {
    const parsed = new URL(url);
    for (const key of [...parsed.searchParams.keys()]) {
      const k = key.toLowerCase();
      if (k.startsWith('utm_') || TRACKING.includes(k)) parsed.searchParams.delete(key);
    }
    return parsed.toString();
  } catch {
    return url;
  }
}

function asLink(text: string): URL | null {
  const candidate = /^www\.[^\s.]+\.\S+$/i.test(text) ? `https://${text}` : text;
  if (!/^https?:\/\/\S+$/i.test(candidate)) return null;
  try {
    const url = new URL(candidate);
    return url.hostname.includes('.') || url.hostname === 'localhost' ? url : null;
  } catch {
    return null;
  }
}

const EMAIL = /^[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9-]+(\.[a-z0-9-]+)*\.[a-z]{2,}$/i;

const digitsOf = (s: string) => s.replace(/\D/g, '');

function asPhone(text: string): string | null {
  if (!/^\+?[\d\s().-]+$/.test(text) || !/^[+(\d]/.test(text)) return null;
  const digits = digitsOf(text);
  if (digits.length < 7 || digits.length > 15) return null;
  // Plain runs of up to eight digits read as one-time codes, not numbers.
  if (/^\d+$/.test(text) && digits.length <= 8) return null;
  // Dates such as 2026-10-16 or 16.10.2026 are not phone numbers.
  // Nor are IP addresses.
  if (/^\d{1,3}(\.\d{1,3}){3}$/.test(text)) return null;
  if (/^\d{4}[-./]\d{1,2}[-./]\d{1,2}$|^\d{1,2}[-./]\d{1,2}[-./]\d{2,4}$/.test(text)) return null;
  return text.trim().startsWith('+') ? `+${digits}` : digits;
}

// ---------- one-time codes ----------

const OTP_WORDS =
  /\b(code|otp|passcode|pin|verification|verify|one[- ]time|2fa|security code|login)\b/i;

function asOtp(text: string): string | null {
  // A lone year reads as a year, not a code.
  if (/^(19|20)\d\d$/.test(text)) return null;
  if (/^\d{4,8}$/.test(text)) return text;
  if (/^\d{3}[ -]\d{3}$/.test(text)) return digitsOf(text);
  return null;
}

/** Finds a 4–8 digit code in a message such as "Your verification code is 482193". */
function otpInMessage(text: string): string | null {
  if (text.length > 400 || !OTP_WORDS.test(text)) return null;
  const match = /(?<![\d.,:/]|\d-)(\d{3}[ -]\d{3}|\d{4,8})(?![\d.,:/-]?\d)/.exec(text);
  return match ? digitsOf(match[1]) : null;
}

// ---------- addresses ----------

// A house number, up to four words, then a street suffix that ends its line or
// comma-separated part, as in "221B Baker Street, London".
const STREET =
  /(?:^|[,\n])\s*\d{1,6}[a-z]?\s+(?:[a-z0-9.'-]+\s+){0,4}(?:street|st|avenue|ave|road|rd|boulevard|blvd|lane|ln|drive|dr|way|court|ct|place|pl|terrace|ter|parkway|pkwy|highway|hwy|circle|cir|square|sq|marg|close|crescent|cres|row|alley|plaza)\.?\s*(?:[,\n]|$)/i;
const POSTAL =
  /\b[A-Z]{2}\s+\d{5}(?:-\d{4})?\b|\b[A-Z]{1,2}\d[A-Z\d]?\s*\d[A-Z]{2}\b|\b\d{3}\s?\d{3}\b|\b\d{5}\b/;

function isAddress(text: string): boolean {
  if (text.length > 240 || text.split('\n').length > 6 || /[{};<>=]/.test(text)) return false;
  if (STREET.test(text)) return true;
  const parts = text.split(/,|\n/).filter((p) => p.trim());
  return parts.length >= 3 && POSTAL.test(text) && /[a-z]{3,}/i.test(text);
}

// ---------- code ----------

const CODE_KEYWORDS =
  /^\s*(import|export|from|const|let|var|function|def|class|return|if|for|while|fn|pub|use|struct|impl|package|public|private|#include|SELECT|INSERT|UPDATE|CREATE|async|await|interface|type)\b/m;

export function looksLikeCode(text: string): boolean {
  const lines = text.split('\n');
  if (lines.length === 1 && text.length > 300) return false;
  let score = 0;
  if (CODE_KEYWORDS.test(text)) score += 2;
  if (/[;{}]\s*$/m.test(text)) score += 1;
  if (/=>|===|!==|&&|\|\||::|->|:=/.test(text)) score += 1;
  if (/\w+\([^()]*\)/.test(text)) score += 1;
  if (lines.length > 1 && lines.some((l) => /^( {2,}|\t)\S/.test(l))) score += 1;
  if (/^\s*(\/\/|#!|\/\*|<!--)/m.test(text)) score += 1;
  // Prose has many words per symbol; code has few.
  const words = (text.match(/[a-z]{2,}/gi) ?? []).length;
  const symbols = (text.match(/[{}()[\];=<>]/g) ?? []).length;
  if (symbols * 4 >= words) score += 1;
  if (/[.!?]\s+[A-Z]/.test(text) && symbols * 4 < words) score -= 2;
  return score >= 3;
}

/** Best-effort language name for a Markdown fence. Empty when unsure. */
export function guessLanguage(text: string): string {
  if (isJson(text)) return 'json';
  if (/^\s*(SELECT|INSERT|UPDATE|DELETE|CREATE|WITH)\b/i.test(text)) return 'sql';
  if (/\bfn\s+\w+\s*\(|\blet\s+mut\b|\bimpl\b|::\w+/.test(text)) return 'rust';
  if (/^\s*def\s+\w+\(.*\):\s*$|^\s*(from\s+\S+\s+)?import\s+\w+\s*$/m.test(text)) return 'python';
  if (/^\s*#include\b/m.test(text)) return 'cpp';
  if (/^\s*<[a-z!][\s\S]*>\s*$/i.test(text)) return 'html';
  if (/^[\s\S]*\{\s*[\w-]+\s*:\s*[^;]+;[\s\S]*\}\s*$/.test(text) && !/\bfunction\b/.test(text))
    return 'css';
  if (/:\s*(string|number|boolean)\b|\binterface\s+\w+|<\w+>\(/.test(text)) return 'ts';
  if (/\b(const|let|var|function)\b|=>/.test(text)) return 'js';
  return '';
}

function isJson(text: string): boolean {
  const t = text.trim();
  if (!/^[[{]/.test(t)) return false;
  try {
    JSON.parse(t);
    return true;
  } catch {
    return false;
  }
}

// ---------- detection ----------

const short = (s: string, n = 28) => (s.length > n ? `${s.slice(0, n - 1)}…` : s);

function linkActions(url: URL, embedded: boolean): ClipAction[] {
  const href = url.toString();
  const clean = cleanLink(href);
  const actions: ClipAction[] = [{ id: 'open', label: 'Open link', type: 'open', value: href }];
  if (embedded) {
    actions.push({ id: 'copy-link', label: 'Copy link', type: 'copy', value: href });
    actions.push({ id: 'save-link', label: 'Save link to Shelf', type: 'save', value: href });
  }
  if (clean !== href)
    actions.push({ id: 'clean', label: 'Copy without tracking', type: 'copy', value: clean });
  actions.push({
    id: 'markdown',
    label: 'Copy as Markdown',
    type: 'copy',
    value: `[${url.hostname.replace(/^www\./, '')}](${clean})`,
  });
  if (!embedded)
    actions.push({
      id: 'domain',
      label: 'Copy domain',
      type: 'copy',
      value: url.hostname.replace(/^www\./, ''),
    });
  return actions;
}

function emailActions(address: string): ClipAction[] {
  return [
    { id: 'mail', label: 'Write email', type: 'open', value: `mailto:${address}` },
    { id: 'copy-email', label: 'Copy address', type: 'copy', value: address },
    {
      id: 'domain',
      label: 'Copy domain',
      type: 'copy',
      value: address.split('@')[1],
    },
  ];
}

function colorActions(color: Rgba, original: string): ClipAction[] {
  const formats = [
    { id: 'hex', label: 'Copy HEX', value: toHex(color) },
    { id: 'rgb', label: 'Copy RGB', value: toRgb(color) },
    { id: 'hsl', label: 'Copy HSL', value: toHsl(color) },
  ];
  const same = (v: string) =>
    v.replace(/\s/g, '').toLowerCase() === original.replace(/\s/g, '').toLowerCase();
  return formats
    .filter((f) => !same(f.value))
    .map((f) => ({ ...f, label: `${f.label} ${f.value}`, type: 'copy' as const }));
}

/** Works out what `input` is and which one-tap actions suit it. */
export function detectClip(input: string | null | undefined): ClipInfo {
  const text = (input ?? '').trim();
  const plain: ClipInfo = { kind: 'text', label: 'Text', actions: [] };
  if (!text || text.length > MAX_SCAN) return plain;
  const singleLine = !text.includes('\n');

  if (singleLine) {
    const color = parseColor(text);
    if (color)
      return { kind: 'color', label: toHex(color), color, actions: colorActions(color, text) };

    const url = asLink(text);
    if (url)
      return {
        kind: 'link',
        label: short(url.hostname.replace(/^www\./, '')),
        actions: linkActions(url, false),
      };

    const email = text.replace(/^mailto:/i, '');
    if (EMAIL.test(email)) return { kind: 'email', label: 'Email', actions: emailActions(email) };

    const otp = asOtp(text);
    if (otp)
      return {
        kind: 'otp',
        label: `Code ${otp}`,
        actions: [{ id: 'copy-code', label: `Copy code ${otp}`, type: 'copy', value: otp }],
      };

    const phone = asPhone(text);
    if (phone)
      return {
        kind: 'phone',
        label: 'Phone',
        actions: [
          { id: 'call', label: 'Call', type: 'open', value: `tel:${phone}` },
          { id: 'copy-digits', label: 'Copy digits only', type: 'copy', value: phone },
        ],
      };
  }

  const code = otpInMessage(text);
  if (code)
    return {
      kind: 'otp',
      label: `Code ${code}`,
      actions: [{ id: 'copy-code', label: `Copy code ${code}`, type: 'copy', value: code }],
    };

  if (isAddress(text)) {
    const oneLine = text
      .split('\n')
      .map((l) => l.trim().replace(/,$/, ''))
      .filter(Boolean)
      .join(', ');
    const actions: ClipAction[] = [
      {
        id: 'map',
        label: 'Open in Maps',
        type: 'open',
        value: `https://www.google.com/maps/search/?api=1&query=${encodeURIComponent(oneLine)}`,
      },
    ];
    if (oneLine !== text)
      actions.push({ id: 'one-line', label: 'Copy on one line', type: 'copy', value: oneLine });
    return { kind: 'address', label: 'Address', actions };
  }

  if (looksLikeCode(text) || isJson(text)) {
    const lang = guessLanguage(text);
    const actions: ClipAction[] = [
      {
        id: 'markdown',
        label: 'Copy as Markdown block',
        type: 'copy',
        value: `\`\`\`${lang}\n${text}\n\`\`\``,
      },
    ];
    if (lang === 'json') {
      const parsed = JSON.parse(text);
      const pretty = JSON.stringify(parsed, null, 2);
      const compact = JSON.stringify(parsed);
      if (pretty !== text)
        actions.push({ id: 'pretty', label: 'Copy formatted', type: 'copy', value: pretty });
      if (compact !== text)
        actions.push({ id: 'minify', label: 'Copy minified', type: 'copy', value: compact });
    }
    return { kind: 'code', label: lang ? `Code · ${lang}` : 'Code', actions };
  }

  // Plain text: offer what is inside it.
  const actions: ClipAction[] = [];
  const found = /\bhttps?:\/\/[^\s<>"')\]]+[^\s<>"')\].,;:!?]/i.exec(text);
  const link = found && asLink(found[0]);
  if (link) actions.push(...linkActions(link, true));
  const mail = /[a-z0-9._%+-]+@[a-z0-9-]+(\.[a-z0-9-]+)*\.[a-z]{2,}/i.exec(text);
  if (mail && !link?.toString().includes(mail[0]))
    actions.push({
      id: 'copy-email',
      label: `Copy ${short(mail[0], 24)}`,
      type: 'copy',
      value: mail[0],
    });
  return { ...plain, actions };
}
