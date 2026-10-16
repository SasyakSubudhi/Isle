import { invoke } from '@tauri-apps/api/core';
import { nativeAvailable } from '../../lib/ipc';
import type { ContextInfo } from '../context/contextLabel';
import type { Habit, Weather } from './widgetLogic';

function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!nativeAvailable) return Promise.reject(new Error('Open the Isle desktop app to use this.'));
  return invoke<T>(command, args);
}

/** Commands for the context-aware pill and the widgets. Kept apart from lib/api. */
export const widgetsApi = {
  context: () => call<ContextInfo | null>('context_current'),
  habits: () => call<Habit[]>('habits_list'),
  saveHabits: (habits: Habit[]) => call<Habit[]>('habits_save', { habits }),
  weather: (refresh = false) => call<Weather>('weather_get', { refresh }),
};
