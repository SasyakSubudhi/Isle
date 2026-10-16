/** Sine path spanning two pill widths so a half-width shift loops seamlessly. */
function wavePath(width: number, period: number, amplitude: number, baseline: number) {
  let d = `M0 ${baseline}`;
  for (let x = 0; x < width; x += period / 2) {
    const dir = (x / (period / 2)) % 2 === 0 ? -1 : 1;
    d += ` Q${x + period / 4} ${baseline + dir * amplitude * 2} ${x + period / 2} ${baseline}`;
  }
  return d;
}
const front = wavePath(296, 74, 2.2, 25);
const back = wavePath(296, 98, 1.6, 22);

/** Ambient layer: flowing waves, drifting motes, a faint sheen and a hairline edge light. CSS only. */
export function Ambient({ quiet, expanded }: { quiet: boolean; expanded: boolean }) {
  return (
    <span className={`ambient ${quiet ? 'is-quiet' : ''} ${expanded ? 'is-expanded' : ''}`} aria-hidden="true">
      <span className="ambient-waves">
        <svg className="ambient-wave wave-back" viewBox="0 0 296 36" preserveAspectRatio="none"><path d={back} /></svg>
        <svg className="ambient-wave wave-front" viewBox="0 0 296 36" preserveAspectRatio="none"><path d={front} /></svg>
      </span>
      <span className="ambient-mote mote-a" /><span className="ambient-mote mote-b" /><span className="ambient-mote mote-c" />
      <span className="ambient-sheen" />
      <span className="ambient-edge" />
    </span>
  );
}
