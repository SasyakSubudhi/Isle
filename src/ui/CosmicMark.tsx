/** Authored vector emblem: an inclined orbit around a luminous central body. */
export function CosmicMark() {
  return (
    <svg className="cosmic-mark" width="23" height="23" viewBox="0 0 32 32" fill="none" aria-hidden="true">
      <defs>
        <radialGradient id="isle-core" cx=".28" cy=".2" r=".85">
          <stop stopColor="#fff5ff" /><stop offset=".38" stopColor="#d9b8ff" />
          <stop offset=".76" stopColor="#9264ed" /><stop offset="1" stopColor="#382257" />
        </radialGradient>
        <linearGradient id="isle-orbit" x1="3" y1="25" x2="29" y2="7" gradientUnits="userSpaceOnUse">
          <stop stopColor="#8361c7" stopOpacity=".15" /><stop offset=".45" stopColor="#c6a0ff" />
          <stop offset=".7" stopColor="#f7e7ff" /><stop offset="1" stopColor="#8f6bd4" stopOpacity=".2" />
        </linearGradient>
      </defs>
      <circle cx="16" cy="16" r="7" fill="url(#isle-core)" />
      <path d="M10 10.3c-4.6 1.6-8 5-6.7 7.5 1.6 3.2 9.2 3.2 16.8-.2 7.5-3.3 11.2-7.2 9.6-9.4-1.2-1.7-4.5-1.9-8.1-.8" stroke="url(#isle-orbit)" strokeWidth="1.3" strokeLinecap="round" />
      <path d="M25.5 23v4M23.5 25h4" stroke="#e7cfff" strokeWidth=".8" strokeLinecap="round" />
      <circle cx="7" cy="6" r=".8" fill="#e5ccff" />
    </svg>
  );
}
