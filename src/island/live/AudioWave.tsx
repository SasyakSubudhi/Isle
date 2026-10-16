import { useEffect, useRef } from 'react';
import { useLive } from './useLiveActivity';

const WIDTH = 296,
  HEIGHT = 36,
  POINTS = 48;

/** A wave whose height follows the recent loudness, newest at the right edge, tapered at both ends. */
export function wavePath(history: number[], phase: number, baseline: number, scale: number) {
  const step = WIDTH / (history.length - 1);
  let d = '';
  history.forEach((level, i) => {
    const x = i * step;
    const taper = Math.sin((Math.PI * i) / (history.length - 1));
    const amplitude = (1.2 + level * 9) * scale * taper;
    const y = baseline + Math.sin(i * 0.55 + phase) * amplitude;
    d += `${i ? ' L' : 'M'}${x.toFixed(1)} ${y.toFixed(2)}`;
  });
  return d;
}

/**
 * Replaces the idle waves while music plays. Reads loudness straight from the
 * store and draws on animation frames, so React never re-renders per sample.
 */
export function AudioWave({ paused }: { paused: boolean }) {
  const front = useRef<SVGPathElement>(null);
  const back = useRef<SVGPathElement>(null);
  const fill = useRef<SVGPathElement>(null);
  useEffect(() => {
    if (paused) return;
    const history = Array.from({ length: POINTS }, () => 0);
    let smooth = 0,
      phase = 0,
      last = performance.now(),
      shift = 0,
      frame = 0;
    const draw = (time: number) => {
      const dt = Math.min(64, time - last);
      last = time;
      const target = useLive.getState().level;
      // Fast attack, slow release, like a VU meter.
      smooth += (target - smooth) * (target > smooth ? 0.45 : 0.08);
      phase -= dt * (0.004 + smooth * 0.006);
      shift += dt;
      if (shift >= 45) {
        shift = 0;
        history.shift();
        history.push(smooth);
      }
      const line = wavePath(history, phase, 20, 1);
      front.current?.setAttribute('d', line);
      fill.current?.setAttribute('d', `${line} L${WIDTH} ${HEIGHT} L0 ${HEIGHT} Z`);
      back.current?.setAttribute('d', wavePath(history, phase * 0.7 + 1.8, 23, 0.6));
      frame = requestAnimationFrame(draw);
    };
    frame = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(frame);
  }, [paused]);
  const flat = wavePath(
    Array.from({ length: POINTS }, () => 0),
    0,
    20,
    1,
  );
  return (
    <span className="audio-wave" aria-hidden="true">
      <svg viewBox={`0 0 ${WIDTH} ${HEIGHT}`} preserveAspectRatio="none">
        <path ref={back} className="audio-wave-back" d={flat} />
        <path
          ref={fill}
          className="audio-wave-fill"
          d={`${flat} L${WIDTH} ${HEIGHT} L0 ${HEIGHT} Z`}
        />
        <path ref={front} className="audio-wave-line" d={flat} />
      </svg>
    </span>
  );
}
