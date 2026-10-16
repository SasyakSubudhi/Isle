import { motion } from 'framer-motion';
import { asset } from '../../lib/api';
import type { ShelfItem, Tab } from '../../lib/types';
import { task } from '../../app/store';
import { Icon } from '../../ui/Icon';
import { IconButton } from '../../ui/primitives';
import { dockApi, dragOut, useDock } from './dock';

const SHOWN = 4;

/** Starts a native drag once the pointer moves a few pixels, so a click stays a click. */
function dragHandle(ids: () => string[]) {
  return (event: React.PointerEvent) => {
    if (event.button !== 0) return;
    event.stopPropagation();
    const { clientX: x, clientY: y } = event;
    const move = (e: PointerEvent) => {
      if (Math.hypot(e.clientX - x, e.clientY - y) <= 6) return;
      stop();
      const files = ids();
      if (files.length) void dragOut(files);
    };
    const stop = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', stop);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', stop, { once: true });
  };
}

function DockTile({ item }: { item: ShelfItem }) {
  return (
    <span
      className={`dock-tile ${item.missing ? 'missing' : ''}`}
      title={
        item.missing
          ? `${item.name} was moved or deleted. Click to take it off the dock.`
          : `Drag ${item.name} into any app`
      }
      onPointerDown={item.missing ? undefined : dragHandle(() => [item.id])}
      onClick={(event) => {
        event.stopPropagation();
        if (item.missing) void task(() => dockApi.remove([item.id]));
      }}
    >
      {item.thumbnail && !item.missing ? (
        <img src={asset(item.thumbnail)} alt="" draggable={false} />
      ) : (
        <Icon name={item.missing ? 'close' : 'file'} size={18} />
      )}
      <span className="dock-tile-name">{item.name}</span>
    </span>
  );
}

/** The compact tray: docked files you can drag straight out of the pill. */
export function DockTray({
  reduced,
  open,
}: {
  reduced: boolean;
  open: (tab?: Tab, focus?: boolean, pin?: boolean) => void;
}) {
  const items = useDock((s) => s.items);
  const ready = items.filter((i) => !i.missing);
  const shown = items.slice(0, SHOWN);
  const extra = items.length - shown.length;
  return (
    <motion.div
      key="dock"
      className="dock-tray"
      initial={{ opacity: 0, y: reduced ? 0 : 4 }}
      animate={{ opacity: 1, y: 0, transition: { delay: reduced ? 0 : 0.06, duration: 0.16 } }}
      exit={{ opacity: 0, transition: { duration: 0.07 } }}
      role="group"
      aria-label="Docked files"
    >
      <span
        className="dock-all"
        title={
          ready.length
            ? `Drag all ${ready.length} file${ready.length === 1 ? '' : 's'} into any app`
            : 'These files were moved or deleted'
        }
        onPointerDown={ready.length ? dragHandle(() => ready.map((i) => i.id)) : undefined}
        onClick={(event) => event.stopPropagation()}
      >
        <Icon name="dock" size={18} />
        <strong>{items.length}</strong>
      </span>
      <span className="dock-row">
        {shown.map((item) => (
          <DockTile key={item.id} item={item} />
        ))}
        {extra > 0 && <span className="dock-more">+{extra}</span>}
      </span>
      <span className="dock-actions" data-no-focus>
        <IconButton
          icon="shelf"
          label="Open Shelf"
          onClick={(event) => {
            event.stopPropagation();
            open('shelf', true, true);
          }}
        />
        <IconButton
          icon="close"
          label="Clear the dock (files stay on Shelf)"
          onClick={(event) => {
            event.stopPropagation();
            void task(dockApi.clear);
          }}
        />
      </span>
    </motion.div>
  );
}

/** What the collapsed pill shows while files are dragged over it. */
export function DockDrop({ reduced }: { reduced: boolean }) {
  const incoming = useDock((s) => s.incoming);
  return (
    <motion.div
      key="drop"
      className="dock-drop"
      initial={{ opacity: 0, scale: reduced ? 1 : 0.96 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, transition: { duration: 0.07 } }}
      transition={{ duration: reduced ? 0.08 : 0.16 }}
      role="status"
    >
      <span className="dock-drop-icon" aria-hidden="true">
        <Icon name="dock" size={18} />
      </span>
      <strong>Drop to dock</strong>
      {incoming > 0 && (
        <small>
          {incoming} file{incoming === 1 ? '' : 's'}
        </small>
      )}
    </motion.div>
  );
}

/** A quiet count on the idle pill while files wait on the dock. */
export function DockBadge({ count }: { count: number }) {
  return (
    <span className="dock-badge" aria-label={`${count} docked file${count === 1 ? '' : 's'}`}>
      {count}
    </span>
  );
}
