import { useEffect } from 'react';
import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { api } from '../../lib/api';
import { nativeAvailable } from '../../lib/ipc';
import type { ShelfItem } from '../../lib/types';
import { task, useApp } from '../../app/store';

/** Files dropped on the pill, newest first, until they are dragged back out. */
interface DockState {
  items: ShelfItem[];
  /** How many files the drag hovering over the pill carries. */
  incoming: number;
}
export const useDock = create<DockState>(() => ({ items: [], incoming: 0 }));

export const dockApi = {
  list: () => invoke<ShelfItem[]>('dock_list'),
  add: (ids: string[]) => invoke<ShelfItem[]>('dock_add', { ids }),
  remove: (ids: string[]) => invoke<ShelfItem[]>('dock_remove', { ids }),
  clear: () => invoke<void>('dock_clear'),
  drag: (ids: string[]) => invoke<{ missing: string[] }>('dock_start_drag', { ids }),
};

/**
 * Drags docked files into another app. A finished drop undocks them on the
 * native side; files moved or deleted since docking are left out and named.
 */
export async function dragOut(ids: string[]) {
  const { dispatch, notify } = useApp.getState();
  dispatch({ type: 'drag', active: true });
  try {
    const result = await task(() => dockApi.drag(ids));
    if (result && result.missing.length)
      notify(
        result.missing.length === 1
          ? `${result.missing[0]} was moved or deleted, so it was left out`
          : `${result.missing.length} files were moved or deleted, so they were left out`,
        true,
      );
  } finally {
    dispatch({ type: 'drag', active: false });
  }
}

/** Keeps the dock in sync with the native side and calls `onDocked` when new files land. */
export function useDockEvents(onDocked: () => void) {
  useEffect(() => {
    if (!nativeAvailable) return;
    let disposed = false;
    const set = (items: ShelfItem[]) => {
      if (disposed) return;
      const before = new Set(useDock.getState().items.map((i) => i.id));
      useDock.setState({ items });
      if (items.some((i) => !before.has(i.id))) onDocked();
    };
    const reload = () =>
      void dockApi
        .list()
        .then((items) => !disposed && useDock.setState({ items }))
        .catch(() => {});
    reload();
    const stops = [
      api.on<ShelfItem[]>('dock://changed', set),
      api.on<number>('dock://drag-count', (incoming) => useDock.setState({ incoming })),
      // Removing or expiring a Shelf item takes it off the dock too.
      api.on('shelf://changed', reload),
    ];
    return () => {
      disposed = true;
      for (const stop of stops) void stop.then((unlisten) => unlisten()).catch(() => {});
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps -- registers once
}
