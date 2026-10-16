import { motion } from 'framer-motion';
import { api, asset, formatClock } from '../lib/api';
import { task, useApp } from '../app/store';
import { IsleMark } from '../ui/IsleMark';
import { ProgressRing } from '../ui/primitives';

/** What the pill shows while collapsed: idle wordmark, a peek notice, the timer or media. */
export function IdlePill({
  reduced,
  paused,
  engaged,
  peek,
  privateMode,
  activeTimer,
  close,
}: {
  reduced: boolean;
  paused: boolean;
  engaged: boolean;
  peek: boolean;
  privateMode: boolean;
  activeTimer: boolean;
  close: () => void;
}) {
  const { island, data, patch } = useApp();
  return (
    <motion.div
      key="idle"
      className="island-content"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, transition: { duration: 0.07 } }}
      whileTap={reduced || paused ? undefined : { scale: 0.97 }}
      transition={{ duration: reduced ? 0.08 : 0.14 }}
    >
      <motion.span
        className="idle-emblem"
        initial={false}
        animate={reduced || paused ? { scale: 1 } : { scale: engaged ? 1.08 : 1 }}
        transition={reduced ? { duration: 0 } : { type: 'spring', stiffness: 420, damping: 26 }}
      >
        {!privateMode && data?.media.playing && data.media.artwork ? (
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
            transition={{ duration: 0.28, ease: [0.2, 0.8, 0.2, 1] }}
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
          <span className="isle-wordmark">isle</span>
          <span className="waveform playing idle-wave" aria-hidden="true">
            {[6, 10, 7].map((n, i) => (
              <span key={i} style={{ height: n, animationDelay: `${i * -0.27}s` }} />
            ))}
          </span>
        </>
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
