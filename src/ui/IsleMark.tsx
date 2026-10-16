export type MarkState = 'idle' | 'notice' | 'timer' | 'media';

/** Minimal status mark: a single dot with a soft breathing halo. State only changes its rhythm. */
export function IsleMark({ state, engaged }: { state: MarkState; engaged: boolean }) {
  return (
    <span className={`isle-mark is-${state} ${engaged ? 'is-engaged' : ''}`} aria-hidden="true">
      <span className="isle-mark-halo" />
      <span className="isle-mark-dot" />
    </span>
  );
}
