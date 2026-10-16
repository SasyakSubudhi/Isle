import { motion } from 'framer-motion';
import { DUR, EASE_OUT } from '../lib/motion';
import { api, asset, formatClock } from '../lib/api';
import { task, useApp } from '../app/store';
import { IsleMark } from '../ui/IsleMark';
import { ProgressRing } from '../ui/primitives';
import { ContextChip, useActivity } from '../modules/context/ContextChip';
import { LiveDetails, LiveGlyph } from './live/LiveActivityRow';
import type { Live } from './live/liveActivity';

/** What the pill shows while collapsed, in priority order: a peek notice, a live activity, the timer, media, your current app, then the idle wordmark. */
export function IdlePill({
  reduced,
  paused,
  engaged,
  peek,
  privateMode,
  activeTimer,
  live,
  now,
  audioWave,
  close,
}: {
  reduced: boolean;
  paused: boolean;
  engaged: boolean;
  peek: boolean;
  privateMode: boolean;
  activeTimer: boolean;
  /** A live activity (call, download, battery) the pill is showing instead of its usual state. */
  live: Live | null;
  now: number;
  /** The audio-reactive wave is running behind the pill, so the small bars are not needed. */
  audioWave: boolean;
  close: () => void;
}) {
  const { island, data, patch } = useApp();
  const activity = useActivity((s) => s.current);
  const showActivity = !!activity && !privateMode && !!data?.settings.context_pill;
  return (
    <motion.div
      key="idle"
      className="island-content"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, transition: { duration: DUR.quick } }}
      whileTap={reduced || paused ? undefined : { scale: 0.97 }}
      transition={{ duration: reduced ? DUR.quick : DUR.base }}
    >
      <motion.span
        className="idle-emblem"
        initial={false}
        animate={reduced || paused ? { scale: 1 } : { scale: engaged ? 1.08 : 1 }}
        transition={reduced ? { duration: 0 } : { type: 'spring', stiffness: 420, damping: 26 }}
      >
        {live && !peek ? (
          <LiveGlyph live={live} />
        ) : !privateMode && data?.media.playing && data.media.artwork ? (
          <img className="idle-album" src={asset(data.media.artwork)} alt="" />
        ) : (
          <IsleMark
            state={
              peek
                ? 'notice'
                : activeTimer
                  ? 'timer'
                  : !privateMode && data?.media.playing
                    ? 'media'
                    : 'idle'
            }
            engaged={engaged}
          />
        )}
      </motion.span>
      {peek ? (
        <>
          <motion.span
            key={island.notice?.title}
            className="peek-copy"
            initial={reduced ? false : { opacity: 0, x: -6, filter: 'blur(3px)' }}
            animate={{ opacity: 1, x: 0, filter: 'blur(0px)' }}
            transition={{ duration: DUR.base, ease: EASE_OUT }}
          >
            {island.notice?.title}
          </motion.span>
          {island.notice?.kind === 'download' && data?.downloads.length ? (
            <button
              className="peek-action"
              onClick={(event) => {
                event.stopPropagation();
                void task(
                  () => api.download(island.notice!.downloadId!, true),
                  'Added to Shelf',
                ).then(async (ok) => {
                  if (!ok) return;
                  const next = await api.bootstrap();
                  patch({ downloads: next.downloads, shelf: next.shelf });
                  close();
                });
              }}
            >
              Add
            </button>
          ) : null}
        </>
      ) : live ? (
        <LiveDetails live={live} now={now} reduced={reduced} />
      ) : activeTimer && data ? (
        <>
          <span className="idle-timer">{formatClock(data.timer.remaining_seconds)}</span>
          <ProgressRing
            size={16}
            value={data.timer.remaining_seconds / data.timer.duration_seconds}
          />
        </>
      ) : !privateMode && data?.media.playing ? (
        <>
          <span className="live-label" title={data.media.title}>
            {data.media.title || 'Now playing'}
          </span>
          {audioWave ? null : (
            <span className="waveform playing idle-wave" aria-hidden="true">
              {[6, 10, 7].map((n, i) => (
                <span key={i} style={{ height: n, animationDelay: `${i * -0.27}s` }} />
              ))}
            </span>
          )}
        </>
      ) : showActivity && activity ? (
        <ContextChip info={activity} />
      ) : (
        <>
          <span className="isle-wordmark">isle</span>
          <span className="idle-tide" aria-hidden="true">
            <span />
            <span />
            <span />
          </span>
        </>
      )}
    </motion.div>
  );
}
