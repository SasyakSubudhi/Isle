import { useEffect, useState } from 'react';
import { api, asset, formatClock } from '../../lib/api';
import { task, useApp } from '../../app/store';
import { Empty, IconButton } from '../../ui/primitives';
import { Icon } from '../../ui/Icon';
import { useReducedMotion } from '../../lib/hooks/useReducedMotion';
import { ReactiveBars } from '../../island/live/ReactiveBars';
export function NowPlayingView() {
  const media = useApp((s) => s.data!.media);
  const systemReduced = useReducedMotion();
  const reduced = useApp((s) => !!s.data?.settings.reduced_motion) || systemReduced;
  const [seek, setSeek] = useState(media.position_seconds);
  const [dragging, setDragging] = useState(false);
  useEffect(() => setSeek(media.position_seconds), [media.position_seconds, media.title]);
  // Windows reports position every couple of seconds; count up in between.
  useEffect(() => {
    if (!media.playing || dragging) return;
    const interval = setInterval(
      () => setSeek((s) => Math.min(s + 1, media.duration_seconds || s + 1)),
      1000,
    );
    return () => clearInterval(interval);
  }, [media.playing, media.duration_seconds, dragging]);
  if (!media.available)
    return (
      <Empty icon="media" title="Your next soundtrack goes here">
        {media.error
          ? 'Windows media controls are unavailable. Open a supported player and try again.'
          : 'Play something in Spotify or YouTube. Track information and controls will appear here.'}
      </Empty>
    );
  return (
    <section className={`feature media-view ${media.accent ? 'has-accent' : ''}`}>
      {media.artwork ? (
        <img className="media-backdrop" src={asset(media.artwork)} alt="" aria-hidden="true" />
      ) : null}
      <div className="now-playing">
        <div className="album-art">
          {media.artwork ? (
            <img src={asset(media.artwork)} alt={`${media.title} artwork`} />
          ) : (
            <Icon name="media" size={36} />
          )}
        </div>
        <div className="track-info">
          <small>{media.playing ? 'NOW PLAYING' : 'PAUSED'}</small>
          <h2>{media.title || 'Untitled track'}</h2>
          <p>{media.artist || media.source}</p>
          {media.playing && !reduced ? (
            <ReactiveBars animate />
          ) : (
            <div className={`waveform ${media.playing ? 'playing' : ''}`} aria-hidden="true">
              {Array.from({ length: 9 }, (_, i) => (
                <span
                  key={i}
                  style={{ animationDelay: `${i * -0.13}s`, height: `${8 + ((i * 7) % 20)}px` }}
                />
              ))}
            </div>
          )}
        </div>
      </div>
      <div className="seek">
        <input
          type="range"
          min={0}
          max={Math.max(1, media.duration_seconds)}
          step={1}
          value={Math.min(seek, media.duration_seconds || 1)}
          disabled={!media.duration_seconds}
          aria-label="Playback position"
          onChange={(e) => setSeek(Number(e.target.value))}
          onPointerDown={() => setDragging(true)}
          onPointerUp={() => {
            setDragging(false);
            void task(() => api.media('seek', seek));
          }}
          onKeyUp={(e) => {
            if (['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(e.key))
              void task(() => api.media('seek', seek));
          }}
        />
        <div>
          <small>{formatClock(seek)}</small>
          <small>{formatClock(media.duration_seconds)}</small>
        </div>
      </div>
      <div className="playback-controls">
        <IconButton
          icon="previous"
          label="Previous track"
          onClick={() => void task(() => api.media('previous'))}
        />
        <button
          className="play-button"
          aria-label={media.playing ? 'Pause playback' : 'Play'}
          onClick={() => void task(() => api.media('toggle'))}
        >
          <Icon name={media.playing ? 'pause' : 'play'} size={24} />
        </button>
        <IconButton
          icon="next"
          label="Next track"
          onClick={() => void task(() => api.media('next'))}
        />
      </div>
    </section>
  );
}
