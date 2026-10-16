import { motion } from 'framer-motion';
import { DUR, EASE_OUT } from '../../lib/motion';
import { fileSize } from '../../lib/api';
import { formatElapsed, formatRate, type Live } from './liveActivity';
import './live.css';

const sensorLabel = {
  screen: 'Sharing screen',
  camera: 'Camera on',
  microphone: 'Mic on',
  call: 'On a call',
};

function Battery({ percent, bolt }: { percent: number; bolt: boolean }) {
  return (
    <svg className="live-battery" viewBox="0 0 16 10" width="16" height="10">
      <rect x="0.5" y="0.5" width="13" height="9" rx="2.5" className="live-battery-case" />
      <rect x="14.2" y="3.2" width="1.3" height="3.6" rx="0.6" className="live-battery-tip" />
      <rect
        x="2"
        y="2"
        width={Math.max(1, (10 * Math.min(100, percent)) / 100)}
        height="6"
        rx="1.2"
        className="live-battery-fill"
      />
      {bolt ? <path d="M7.6 1.6 5.2 5.4h2l-.8 3 2.6-4h-2z" className="live-battery-bolt" /> : null}
    </svg>
  );
}

/** Left-hand mark for a live activity, drawn inside the idle mark's slot. */
export function LiveGlyph({ live }: { live: Live }) {
  if (live.kind === 'download')
    return (
      <svg viewBox="0 0 14 14" width="14" height="14" className="live-arrow">
        <path d="M7 2.5v7M3.8 6.6 7 9.8l3.2-3.2M3 12h8" />
      </svg>
    );
  if (live.kind === 'power' || live.kind === 'battery-low')
    return <Battery percent={live.percent} bolt={live.kind === 'power' && live.pluggedIn} />;
  return <span className="live-dot" />;
}

/** Label and trailing detail for a live activity. */
export function LiveDetails({ live, now, reduced }: { live: Live; now: number; reduced: boolean }) {
  let label: string, detail: string;
  switch (live.kind) {
    case 'download':
      label = live.count > 1 ? `${live.count} downloads` : live.name;
      detail = formatRate(live.rate) || fileSize(live.bytes);
      break;
    case 'power':
      label = live.pluggedIn ? 'Charging' : 'On battery';
      detail = `${live.percent}%`;
      break;
    case 'battery-low':
      label = live.critical ? 'Plug in soon' : 'Battery low';
      detail = `${live.percent}%`;
      break;
    default:
      label = `${sensorLabel[live.kind]} · ${live.app}`;
      detail = formatElapsed(live.since, now);
  }
  return (
    <>
      <motion.span
        key={`${live.kind}:${label}`}
        className={`live-label is-${live.kind} ${live.kind === 'battery-low' && live.critical ? 'is-critical' : ''}`}
        initial={reduced ? false : { opacity: 0, y: 3 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: DUR.base, ease: EASE_OUT }}
        title={label}
      >
        {label}
      </motion.span>
      {detail ? <span className="live-detail">{detail}</span> : null}
      {live.kind === 'download' ? <span className="live-progress" aria-hidden="true" /> : null}
      {/* Announce what changed, not every tick of the call timer or speed. */}
      <span className="sr-only" role="status">
        {label}
      </span>
    </>
  );
}
