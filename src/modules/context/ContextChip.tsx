import { useEffect, useState } from 'react';
import { create } from 'zustand';
import { api } from '../../lib/api';
import { nativeAvailable } from '../../lib/ipc';
import { widgetsApi } from '../widgets/widgetsApi';
import {
  contextLabel,
  formatElapsed,
  formatMeeting,
  tickMs,
  type ContextInfo,
  type ContextKind,
} from './contextLabel';

/** The current activity, kept outside the pill so collapsing and expanding doesn't lose it. */
export const useActivity = create<{ current: ContextInfo | null }>(() => ({ current: null }));

/** Starts listening once for the whole app. Call from the shell. */
export function useActivityFeed(enabled: boolean) {
  useEffect(() => {
    if (!nativeAvailable || !enabled) {
      useActivity.setState({ current: null });
      return;
    }
    let disposed = false;
    void widgetsApi
      .context()
      .then((current) => {
        if (!disposed) useActivity.setState({ current });
      })
      .catch(() => {});
    const stop = api.on<ContextInfo | null>('context://changed', (current) => {
      if (!disposed) useActivity.setState({ current });
    });
    return () => {
      disposed = true;
      void stop.then((unlisten) => unlisten()).catch(() => {});
    };
  }, [enabled]);
}

const glyphs: Record<ContextKind, string> = {
  code: 'M9 7l-5 5 5 5 M15 7l5 5-5 5',
  meeting: 'M3 7h12v10H3z M15 11l6-3v8l-6-3',
  reading: 'M4 5h6c2 0 2 2 2 2s0-2 2-2h6v14h-6c-2 0-2 2-2 2s0-2-2-2H4z',
  writing: 'M4 20l4-1 11-11-3-3L5 16z M14 7l3 3',
};

/** Pill content while you're coding, in a meeting, reading or writing. */
export function ContextChip({ info }: { info: ContextInfo }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    setNow(Date.now());
    const interval = setInterval(() => setNow(Date.now()), tickMs(info.kind));
    return () => clearInterval(interval);
  }, [info.kind, info.since]);
  const elapsed = now - info.since;
  const time = info.kind === 'meeting' ? formatMeeting(elapsed) : formatElapsed(elapsed);
  return (
    <span
      className={`context-chip is-${info.kind}`}
      title={`${contextLabel(info.kind)} in ${info.app} · ${time}`}
    >
      <svg
        className="context-glyph"
        width={13}
        height={13}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
      >
        <path d={glyphs[info.kind]} />
      </svg>
      <span className="context-label">{contextLabel(info.kind)}</span>
      <span className="context-time">{time}</span>
    </span>
  );
}
