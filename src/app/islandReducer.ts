import type { Tab } from '../lib/types';
export interface IslandState {
  /** `dock` is the compact tray of files dropped on the pill. */
  mode: 'idle' | 'peek' | 'expanded' | 'dock';
  tab: Tab;
  pinned: boolean;
  /** Isle is dragging files out into another app. */
  dragging: boolean;
  /** Files from another app are being dragged over the island. */
  dropHover: boolean;
  notice: { title: string; tab: Tab; appId?: string; downloadId?: string; kind?: string } | null;
}
export type Action =
  | { type: 'open'; tab?: Tab; pin?: boolean }
  | { type: 'close' }
  | { type: 'toggle' }
  | { type: 'pin' }
  | { type: 'drag'; active: boolean }
  | { type: 'dropHover'; active: boolean }
  | { type: 'dock' }
  | { type: 'peek'; notice: NonNullable<IslandState['notice']> };
export const initialIsland: IslandState = {
  mode: 'idle',
  tab: 'shelf',
  pinned: false,
  dragging: false,
  dropHover: false,
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
      return { ...state, dragging: action.active };
    case 'dropHover':
      // The collapsed pill reacts in place; an open panel switches to Shelf.
      if (state.dropHover === action.active) return state;
      return {
        ...state,
        dropHover: action.active,
        ...(action.active && state.mode === 'expanded' ? { tab: 'shelf' as const } : {}),
      };
    case 'dock':
      return state.mode === 'expanded' ? state : { ...state, mode: 'dock' };
    case 'peek':
      return {
        ...state,
        notice: action.notice,
        mode: state.mode === 'expanded' || state.mode === 'dock' ? state.mode : 'peek',
      };
  }
}
