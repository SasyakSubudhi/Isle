import { useEffect, type Dispatch, type SetStateAction } from 'react';
import { ipc, nativeAvailable, type TaskbarSlot } from '../lib/ipc';
import { api } from '../lib/api';
import type { Download, FeatureSettings, MediaState, ShelfItem, Tab, Timer } from '../lib/types';
import { task, useApp } from '../app/store';
import { dockApi } from '../modules/dock/dock';
import { dndActive } from '../lib/dnd';
import { openPalette } from './palette/CommandPalette';
import { useLive } from './live/useLiveActivity';

/** Peeks stay away in presentation mode and during Do Not Disturb. */
const quiet = () => {
  const settings = useApp.getState().data?.settings;
  return !!settings?.presentation_mode || dndActive(settings, useLive.getState().activity);
};

export interface IsleControls {
  open: (tab?: Tab, focus?: boolean, pin?: boolean) => void;
  close: () => void;
  enter: () => void;
  exit: () => void;
  cancelTimers: () => void;
  setSlot: Dispatch<SetStateAction<TaskbarSlot | null>>;
  setPageVisible: Dispatch<SetStateAction<boolean>>;
  setNativeVisible: Dispatch<SetStateAction<boolean>>;
  setSystemLight: Dispatch<SetStateAction<boolean>>;
}

/**
 * Every event Isle listens to: native window and taskbar events, hotkeys,
 * Shelf, downloads, timer, media and settings updates, plus the toast and
 * peek auto-dismiss timers.
 */
export function useIsleEvents(controls: IsleControls) {
  const { toast, island } = useApp();
  useEffect(() => {
    const { load, dispatch, patch, notify, refreshShelf } = useApp.getState();
    const {
      open,
      close,
      enter,
      exit,
      cancelTimers,
      setSlot,
      setPageVisible,
      setNativeVisible,
      setSystemLight,
    } = controls;
    void load().then(() => {
      if (useApp.getState().data?.open_on_start) open('shelf', true, true);
    });
    const update = () => setPageVisible(!document.hidden);
    document.addEventListener('visibilitychange', update);
    const scheme = matchMedia('(prefers-color-scheme: light)');
    const theme = () => setSystemLight(scheme.matches);
    scheme.addEventListener('change', theme);
    if (!nativeAvailable)
      return () => {
        document.removeEventListener('visibilitychange', update);
        scheme.removeEventListener('change', theme);
      };
    let disposed = false;
    const stops: Promise<() => void>[] = [];
    const watch = <T>(event: string, handler: (payload: T) => void) =>
      stops.push(
        api.on<T>(event, (payload) => {
          if (!disposed) handler(payload);
        }),
      );
    stops.push(
      ipc.onDisplayChanged((value) => {
        if (!disposed) setSlot(value);
      }),
    );
    void ipc
      .getTaskbarSlot()
      .then((value) => {
        if (!disposed) setSlot(value);
      })
      .catch((e) => notify(String(e), true));
    watch<boolean>('island://visibility', setNativeVisible);
    watch<boolean>('island://pointer', (over) => (over ? enter() : exit()));
    watch<boolean>('shelf://native-drop-hover', (active) =>
      dispatch({ type: 'dropHover', active }),
    );
    watch<string>('island://hotkey', (name) => {
      if (name === 'palette') {
        open(undefined, true, true);
        openPalette();
      } else if (name === 'toggle') {
        if (useApp.getState().island.mode === 'expanded') close();
        else open(undefined, true, true);
      } else
        open(
          name === 'usage'
            ? 'usage'
            : name === 'dictionary'
              ? 'dictionary'
              : name === 'settings'
                ? 'settings'
                : 'shelf',
          true,
          true,
        );
    });
    watch<string>('dictionary://selection', (text) => {
      useApp.setState({ selection: text });
      open('dictionary', true, true);
    });
    watch<ShelfItem>('shelf://item-added', (item) => {
      void refreshShelf();
      if (!quiet())
        dispatch({
          type: 'peek',
          notice: {
            title: item.history ? 'Clipboard updated' : `${item.name} added`,
            tab: 'shelf',
          },
        });
    });
    watch('shelf://changed', () => void refreshShelf());
    watch<Download>('download://detected', (item) => {
      const current = useApp.getState().data;
      if (current) {
        patch({ downloads: [...current.downloads.filter((d) => d.id !== item.id), item] });
        if (!quiet())
          dispatch({
            type: 'peek',
            notice: { title: item.name, tab: 'shelf', kind: 'download', downloadId: item.id },
          });
      }
    });
    watch<string>('download://error', (error) => notify(error, true));
    watch<string>('app://error', (error) => notify(error, true));
    watch<FeatureSettings>('settings://changed', (settings) => patch({ settings }));
    watch<MediaState>('media://changed', (media) => patch({ media }));
    watch<Timer>('timer://changed', (timer) => patch({ timer }));
    watch<Timer>('timer://completed', (timer) => {
      patch({ timer });
      if (!quiet())
        dispatch({
          type: 'peek',
          notice: {
            title:
              timer.phase === 'focus'
                ? 'Focus complete. Time for a break.'
                : 'Break complete. Ready when you are.',
            tab: 'focus',
          },
        });
    });
    watch<{ id: string; name: string; minutes: number }>('usage://limit', (item) => {
      if (!quiet())
        dispatch({
          type: 'peek',
          notice: {
            title: `${item.name} reached ${item.minutes} min`,
            tab: 'usage',
            kind: 'limit',
            appId: item.id,
          },
        });
    });
    watch<string>('ai://warning', (title) => {
      if (!quiet()) dispatch({ type: 'peek', notice: { title, tab: 'ai' } });
    });
    watch<boolean>('capture://state', (value) => useApp.setState({ capturing: value }));
    watch('shelf://drag-finished', () => dispatch({ type: 'drag', active: false }));
    stops.push(
      api.onDrop(
        (paths) => {
          open('shelf', false, true);
          void task(() => api.addFiles(paths)).then((items) => {
            if (items) void dockApi.add(items.map((i) => i.id)).catch(() => {});
            return refreshShelf();
          });
        },
        (active) => dispatch({ type: 'dropHover', active }),
      ),
    );
    for (const stop of stops) void stop.catch((e) => notify(String(e), true));
    return () => {
      disposed = true;
      document.removeEventListener('visibilitychange', update);
      scheme.removeEventListener('change', theme);
      for (const stop of stops) void stop.then((unlisten) => unlisten()).catch(() => {});
      cancelTimers();
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps -- registers once, like before
  useEffect(() => {
    if (!toast) return;
    const timeout = setTimeout(() => useApp.setState({ toast: null }), toast.error ? 6500 : 2400);
    return () => clearTimeout(timeout);
  }, [toast]);
  useEffect(() => {
    if (island.mode !== 'peek') return;
    const timeout = setTimeout(() => {
      if (useApp.getState().island.mode === 'peek') controls.close();
    }, 5500);
    return () => clearTimeout(timeout);
  }, [island.mode, island.notice]); // eslint-disable-line react-hooks/exhaustive-deps
}
