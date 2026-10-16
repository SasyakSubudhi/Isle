import { useSyncExternalStore } from 'react';

const preference = '(prefers-reduced-motion: reduce)';

function subscribe(onChange: () => void) {
  const media = window.matchMedia(preference);
  media.addEventListener('change', onChange);
  return () => media.removeEventListener('change', onChange);
}

function getSnapshot() {
  return window.matchMedia(preference).matches;
}

// React's external-store subscription updates immediately when the OS preference
// changes, including after the pill has already mounted.
export function useReducedMotion() {
  return useSyncExternalStore(subscribe, getSnapshot, () => true);
}
