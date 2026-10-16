import { useEffect, useRef } from 'react';
import { useLive } from './useLiveActivity';

/** Bars that follow the output loudness, each a little behind the last, like a ripple. */
export function ReactiveBars({ count = 9, animate }: { count?: number; animate: boolean }) {
  const bars = useRef<(HTMLSpanElement | null)[]>([]);
  useEffect(() => {
    if (!animate) return;
    const levels = Array.from({ length: count }, () => 0);
    let frame = 0,
      t = 0;
    const draw = () => {
      t += 1;
      const target = useLive.getState().level;
      levels.forEach((level, i) => {
        const wobble = 0.75 + 0.25 * Math.sin(t * 0.09 + i * 1.3);
        const goal = target * wobble;
        // Bars further from the centre react more slowly.
        const speed = 0.35 - Math.abs(i - (count - 1) / 2) * 0.04;
        levels[i] = level + (goal - level) * (goal > level ? speed : speed * 0.3);
        const bar = bars.current[i];
        if (bar) bar.style.transform = `scaleY(${(0.2 + levels[i] * 0.8).toFixed(3)})`;
      });
      frame = requestAnimationFrame(draw);
    };
    frame = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(frame);
  }, [animate, count]);
  return (
    <div className="waveform reactive-bars" aria-hidden="true">
      {Array.from({ length: count }, (_, i) => (
        <span
          key={i}
          ref={(el) => {
            bars.current[i] = el;
          }}
          style={{ height: `${8 + ((i * 7) % 20)}px` }}
        />
      ))}
    </div>
  );
}
