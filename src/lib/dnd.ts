import type { FeatureSettings } from './types';

type Sensors = { sensors: { kind: string }[] } | null | undefined;

/** True while a call is going on: the camera, microphone or screen sharing is in use. */
export const inCall = (activity: Sensors) =>
  !!activity?.sensors.some((s) => ['camera', 'microphone', 'screen'].includes(s.kind));

/**
 * Do Not Disturb: switched on by hand, or automatically during a call when
 * `auto_dnd` is on. Presentation mode is stricter and handled separately.
 */
export const dndActive = (
  settings: Pick<FeatureSettings, 'do_not_disturb' | 'auto_dnd'> | undefined,
  activity: Sensors,
) => !!settings && (settings.do_not_disturb || (settings.auto_dnd && inCall(activity)));
