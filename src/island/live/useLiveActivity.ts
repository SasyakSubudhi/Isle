import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { create } from 'zustand';
import { api } from '../../lib/api';
import { nativeAvailable } from '../../lib/ipc';
import { pickLive, POWER_NOTICE_MS, type Live, type LiveActivity } from './liveActivity';

interface LiveState {
  activity: LiveActivity | null;
  powerChangedAt: number | null;
  /** Raw output loudness, 0..1, while the meter is on. */
  level: number;
}
export const useLive = create<LiveState>(() => ({
  activity: null,
  powerChangedAt: null,
  level: 0,
}));

function receive(next: LiveActivity) {
  const before = useLive.getState().activity?.battery;
  const changed =
    !!before && !!next.battery && before.plugged_in !== next.battery.plugged_in
      ? Date.now()
      : useLive.getState().powerChangedAt;
  useLive.setState({ activity: next, powerChangedAt: changed });
}

/**
 * Listens to live activity and the audio meter, and returns what the pill
 * should show. The meter only runs while music plays and motion is allowed.
 */
export function useLiveActivity({
  privateMode,
  timerActive,
  mediaPlaying,
  animate,
}: {
  privateMode: boolean;
  timerActive: boolean;
  mediaPlaying: boolean;
  animate: boolean;
}): { live: Live | null; now: number } {
  // Select fields so the 25 Hz loudness updates never re-render the shell.
  const activity = useLive((s) => s.activity);
  const powerChangedAt = useLive((s) => s.powerChangedAt);
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!nativeAvailable) return;
    let disposed = false;
    void invoke<LiveActivity>('activity_get')
      .then((value) => {
        if (!disposed) receive(value);
      })
      .catch(() => {});
    const stops = [
      api.on<LiveActivity>('activity://changed', (value) => {
        if (!disposed) receive(value);
      }),
      api.on<number>('audio://level', (level) => {
        if (!disposed) useLive.setState({ level });
      }),
    ];
    return () => {
      disposed = true;
      for (const stop of stops) void stop.then((unlisten) => unlisten()).catch(() => {});
    };
  }, []);
  const meter = mediaPlaying && !privateMode && animate;
  useEffect(() => {
    if (!nativeAvailable) return;
    void invoke('audio_meter', { enabled: meter }).catch(() => {});
    if (!meter) useLive.setState({ level: 0 });
  }, [meter]);
  useEffect(
    () => () => {
      if (nativeAvailable) void invoke('audio_meter', { enabled: false }).catch(() => {});
    },
    [],
  );
  const live = pickLive({ activity, powerChangedAt, now, timerActive, mediaPlaying, privateMode });
  // Tick once a second for call timers, and to end the charger notice on time.
  const ticking =
    live?.kind === 'call' ||
    live?.kind === 'camera' ||
    live?.kind === 'microphone' ||
    live?.kind === 'screen' ||
    (powerChangedAt !== null && now - powerChangedAt < POWER_NOTICE_MS);
  useEffect(() => {
    if (!ticking) return;
    setNow(Date.now());
    const interval = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(interval);
  }, [ticking]);
  return { live, now };
}
