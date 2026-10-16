/** Sine path spanning two pill widths so a half-width shift loops seamlessly. */
function wavePath(width: number, period: number, amplitude: number, baseline: number) {
  let d = `M0 ${baseline}`;
  for (let x = 0; x < width; x += period / 2) {
    const dir = (x / (period / 2)) % 2 === 0 ? -1 : 1;
    d += ` Q${x + period / 4} ${baseline + dir * amplitude * 2} ${x + period / 2} ${baseline}`;
  }
  return d;
}
const layers = [
  { key: 'back', line: wavePath(296, 74 * 2, 2.6, 17) },
  { key: 'mid', line: wavePath(296, 74, 2.4, 22) },
  { key: 'front', line: wavePath(296, ((74 / 2) * 3) / 1.5, 1.8, 27) },
];

/** Ambient layer: layered waves that fill the pill, drifting motes, a sheen and an edge light. CSS only. */
export function Ambient({ quiet, expanded }: { quiet: boolean; expanded: boolean }) {
  return (
    <span
      className={`ambient ${quiet ? 'is-quiet' : ''} ${expanded ? 'is-expanded' : ''}`}
      aria-hidden="true"
    >
      <span className="ambient-waves">
        {layers.map((layer) => (
          <svg
            key={layer.key}
            className={`ambient-wave wave-${layer.key}`}
            viewBox="0 0 296 36"
            preserveAspectRatio="none"
          >
            <path className="wave-fill" d={`${layer.line} L296 36 L0 36 Z`} />
            <path className="wave-line" d={layer.line} />
          </svg>
        ))}
      </span>
      {['a', 'b', 'c', 'd', 'e', 'f'].map((id) => (
        <span key={id} className={`ambient-mote mote-${id}`} />
      ))}
      <span className="ambient-sheen" />
      <span className="ambient-edge" />
    </span>
  );
}
