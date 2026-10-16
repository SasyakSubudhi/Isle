import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface PhysicalRect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}
export interface TaskbarSlot {
  window_x: number;
  window_y: number;
  anchor_x: number;
  anchor_y: number;
  scale: number;
  monitor: string;
  monitor_rect: PhysicalRect;
  source: 'taskbar' | 'top-center' | 'custom';
  reason: string | null;
  taskbar: PhysicalRect | null;
  taskbar_hidden: boolean;
  gap: [number, number] | null;
  pill_width: number;
}
export interface WindowPreferences {
  position_mode: 'taskbar' | 'top-center' | 'custom';
  monitor: string | null;
  pill_width: number;
  custom_x: number;
  custom_y: number;
}
export interface HitRegion {
  x: number;
  y: number;
  width: number;
  height: number;
}

export const nativeAvailable = isTauri();
export const ipc = {
  getTaskbarSlot: () => invoke<TaskbarSlot | null>('get_taskbar_slot'),
  reportHitRegion: (region: HitRegion) => invoke<void>('report_hit_region', { region }),
  setFocusable: (enabled: boolean) => invoke<void>('set_focusable', { enabled }),
  positionGet: () => invoke<WindowPreferences>('position_get'),
  positionSet: (preferences: WindowPreferences) => invoke<void>('position_set', { preferences }),
  startCustomDrag: () => invoke<void>('start_custom_drag'),
  openPositionMenu: () => invoke<void>('open_position_menu'),
  onDisplayChanged: (handler: (slot: TaskbarSlot) => void) =>
    listen<TaskbarSlot>('display://changed', (event) => handler(event.payload)),
  onVisibilityChanged: (handler: (visible: boolean) => void) =>
    listen<boolean>('island://visibility', (event) => handler(event.payload)),
};
