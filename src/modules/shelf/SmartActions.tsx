import { useMemo } from 'react';
import { api } from '../../lib/api';
import { detectClip, type ClipAction, type ClipInfo, type ClipKind } from '../../lib/clipSmart';
import type { ShelfItem } from '../../lib/types';
import { task } from '../../app/store';
import { Icon, type IconName } from '../../ui/Icon';
import '../../styles/smart-clip.css';

const NONE: ClipInfo = { kind: 'text', label: 'Text', actions: [] };

/** What a text or link item is, and the actions that fit it. */
export function useClipInfo(item: ShelfItem): ClipInfo {
  const textual = item.kind === 'text' || item.kind === 'link';
  return useMemo(() => (textual ? detectClip(item.content) : NONE), [textual, item.content]);
}

// Glyphs Icon doesn't carry, kept here so the shared icon set stays untouched.
const glyphs: Partial<Record<ClipKind, string>> = {
  code: 'M8 7l-5 5 5 5 M16 7l5 5-5 5 M13.5 5l-3 14',
  otp: 'M4 9h16v10H4z M8 9V6a4 4 0 0 1 8 0v3 M9 14h.01 M12 14h.01 M15 14h.01',
  phone:
    'M5 4h4l2 5-2.5 1.5a11 11 0 0 0 5 5L15 13l5 2v4a2 2 0 0 1-2 2A16 16 0 0 1 3 6a2 2 0 0 1 2-2',
  address:
    'M12 21s-7-6.5-7-12a7 7 0 0 1 14 0c0 5.5-7 12-7 12z M12 11.5a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5',
};
const iconFor: Partial<Record<ClipKind, IconName>> = { link: 'link', email: 'mail' };

function Glyph({ kind, size }: { kind: ClipKind; size: number }) {
  const icon = iconFor[kind];
  if (icon) return <Icon name={icon} size={size} />;
  const d = glyphs[kind];
  if (!d) return <Icon name="notes" size={size} />;
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={d} />
    </svg>
  );
}

/** Tile artwork: a swatch for colors, a kind glyph for everything else. */
export function ClipArt({ info, size = 22 }: { info: ClipInfo; size?: number }) {
  if (info.color) {
    const { r, g, b, a } = info.color;
    return (
      <span className="clip-swatch" style={{ backgroundColor: `rgba(${r}, ${g}, ${b}, ${a})` }} />
    );
  }
  return <Glyph kind={info.kind} size={size} />;
}

export function runClipAction(action: ClipAction) {
  if (action.type === 'open') return task(() => api.clipOpenUrl(action.value));
  if (action.type === 'save') return task(() => api.addText(action.value), 'Saved to Shelf');
  return task(() => api.clipWriteText(action.value), 'Copied');
}

const actionIcon = (a: ClipAction): IconName =>
  a.type === 'open'
    ? a.value.startsWith('mailto:')
      ? 'mail'
      : 'link'
    : a.type === 'save'
      ? 'pin'
      : 'copy';

/** The one action a tile offers next to its menu, when the item has a clear type. */
export function QuickClipAction({ info }: { info: ClipInfo }) {
  const action = info.kind === 'text' ? undefined : info.actions[0];
  if (!action) return null;
  return (
    <button
      type="button"
      className="icon-button clip-quick"
      aria-label={action.label}
      title={action.label}
      onClick={(e) => {
        e.stopPropagation();
        void runClipAction(action);
      }}
    >
      <Icon name={actionIcon(action)} />
    </button>
  );
}

/** Every action for the open item, shown in the Shelf detail view. */
export function SmartActions({ item }: { item: ShelfItem }) {
  const info = useClipInfo(item);
  // Link items already have an Open button in the row above.
  const actions = info.actions.filter((a) => !(item.kind === 'link' && a.id === 'open'));
  if (!actions.length) return null;
  return (
    <div className="clip-actions">
      <small className="clip-kind">
        <ClipArt info={info} size={12} />
        {info.kind === 'text' ? 'Found in this text' : info.label}
      </small>
      <div className="action-row">
        {actions.map((action) => (
          <button key={action.id} title={action.value} onClick={() => void runClipAction(action)}>
            <Icon name={actionIcon(action)} />
            {action.label}
          </button>
        ))}
      </div>
    </div>
  );
}
