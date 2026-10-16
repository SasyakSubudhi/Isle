/** Ambient layer: a faint sheen that drifts across the pill and a hairline edge light. CSS only. */
export function Ambient({ quiet, expanded }: { quiet: boolean; expanded: boolean }) {
  return (
    <span className={`ambient ${quiet ? 'is-quiet' : ''} ${expanded ? 'is-expanded' : ''}`} aria-hidden="true">
      <span className="ambient-sheen" />
      <span className="ambient-edge" />
    </span>
  );
}
