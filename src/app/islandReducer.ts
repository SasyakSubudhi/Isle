import type { Tab } from '../lib/types';
export interface IslandState {
  mode: 'idle' | 'peek' | 'expanded';
  tab: Tab;
  pinned: boolean;
  dragging: boolean;
  notice: { title: string; tab: Tab; appId?: string; downloadId?: string; kind?: string } | null;
}
export type Action =
  | { type: 'open'; tab?: Tab; pin?: boolean }
  | { type: 'close' }
  | { type: 'toggle' }
  | { type: 'pin' }
  | { type: 'drag'; active: boolean }
  | { type: 'peek'; notice: NonNullable<IslandState['notice']> };
export const initialIsland: IslandState = {
  mode: 'idle',
  tab: 'shelf',
  pinned: false,
  dragging: false,
  notice: null,
};
export function reduceIsland(state: IslandState, action: Action): IslandState {
  switch (action.type) {
    case 'open':
      return {
        ...state,
        mode: 'expanded',
        tab: action.tab ?? state.tab,
        pinned: action.pin ?? state.pinned,
      };
    case 'close':
      return { ...state, mode: 'idle', pinned: false, notice: null };
    case 'toggle':
      return state.mode === 'expanded'
        ? reduceIsland(state, { type: 'close' })
        : reduceIsland(state, { type: 'open', pin: true });
    case 'pin':
      return { ...state, pinned: !state.pinned };
    case 'drag':
      return {
        ...state,
        dragging: action.active,
        ...(action.active ? { mode: 'expanded' as const, tab: 'shelf' as const } : {}),
      };
    case 'peek':
      return {
        ...state,
        notice: action.notice,
        mode: state.mode === 'expanded' ? 'expanded' : 'peek',
      };
  }
}
