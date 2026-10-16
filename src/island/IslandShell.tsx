import { useLayoutEffect, useEffect, useRef, useState } from 'react';
import { AnimatePresence, motion } from 'framer-motion';
import { useReducedMotion } from '../lib/hooks/useReducedMotion';
import { ipc, nativeAvailable, type TaskbarSlot } from '../lib/ipc';
import { api } from '../lib/api';
import type { Tab } from '../lib/types';
import { task, useApp } from '../app/store';
import { Ambient } from './Ambient';
import { IdlePill } from './IdlePill';
import { Panel } from './Panel';
import { tabs } from './islandTabs';
import { useIsleEvents } from './useIsleEvents';
import { DockBadge, DockDrop, DockTray } from '../modules/dock/DockViews';
import { useDock, useDockEvents } from '../modules/dock/dock';

export function IslandShell() {
  const { island, data, dispatch, notify, refreshShelf } = useApp();
  const shell = useRef<HTMLDivElement>(null);
  const [slot, setSlot] = useState<TaskbarSlot | null>(null);
  const [pageVisible, setPageVisible] = useState(() => !document.hidden);
  const [nativeVisible, setNativeVisible] = useState(true);
  const [engaged, setEngaged] = useState(false);
  const systemReduced = useReducedMotion();
  const [systemLight, setSystemLight] = useState(
    () => matchMedia('(prefers-color-scheme: light)').matches,
  );
  const reduced = systemReduced || !!data?.settings.reduced_motion;
  const privateMode = !!data?.settings.presentation_mode;
  const expanded = island.mode === 'expanded';
  const peek = island.mode === 'peek' && !privateMode;
  const docked = useDock((s) => s.items.length);
  const dock = island.mode === 'dock' && !privateMode && docked > 0;
  const dropping = island.dropHover && !expanded;
  const paused = !pageVisible || !nativeVisible;
  const leave = useRef<ReturnType<typeof setTimeout> | null>(null);
  const hover = useRef<ReturnType<typeof setTimeout> | null>(null);
  const hitFrame = useRef<number | null>(null);
  const lastBounds = useRef('');
  const close = () => {
    dispatch({ type: 'close' });
    if (nativeAvailable) void ipc.setFocusable(false).catch((e) => notify(String(e), true));
  };
  const open = (tab?: Tab, focus = true, pin?: boolean) => {
    if (leave.current) clearTimeout(leave.current);
    if (hover.current) clearTimeout(hover.current);
    const show = () => dispatch({ type: 'open', tab, pin });
    if (nativeAvailable && focus)
      void ipc
        .setFocusable(true)
        .then(show)
        .catch((e) => {
          show();
          notify(String(e), true);
        });
    else show();
  };
  const enter = () => {
    setEngaged(true);
    if (leave.current) clearTimeout(leave.current);
    const current = useApp.getState();
    // Files waiting on the dock take priority over hover-to-expand.
    if (
      current.island.mode === 'idle' &&
      useDock.getState().items.length &&
      !current.data?.settings.presentation_mode
    ) {
      if (hover.current) clearTimeout(hover.current);
      hover.current = setTimeout(() => dispatch({ type: 'dock' }), 150);
    } else if (current.island.mode !== 'expanded' && current.data?.settings.hover_expand) {
      if (hover.current) clearTimeout(hover.current);
      hover.current = setTimeout(() => open(undefined, false), 150);
    }
  };
  const exit = () => {
    setEngaged(false);
    if (hover.current) clearTimeout(hover.current);
    if (leave.current) clearTimeout(leave.current);
    leave.current = setTimeout(() => {
      const current = useApp.getState();
      if (
        !current.island.pinned &&
        !current.island.dragging &&
        !current.island.dropHover &&
        !current.capturing &&
        !(document.hasFocus() && document.activeElement?.matches('input,textarea,select'))
      )
        close();
    }, 600);
  };
  useIsleEvents({
    open,
    close,
    enter,
    exit,
    cancelTimers: () => {
      if (hover.current) clearTimeout(hover.current);
      if (leave.current) clearTimeout(leave.current);
    },
    setSlot,
    setPageVisible,
    setNativeVisible,
    setSystemLight,
  });
  useEffect(() => {
    // Everything was dragged out or cleared: fold the tray back into the pill.
    if (island.mode === 'dock' && !dock && !island.dragging) close();
  }, [island.mode, dock, island.dragging]); // eslint-disable-line react-hooks/exhaustive-deps
  useDockEvents(() => {
    if (!useApp.getState().data?.settings.presentation_mode) dispatch({ type: 'dock' });
  });
  const report = () => {
    if (!nativeAvailable || !shell.current) return;
    const r = shell.current.getBoundingClientRect();
    const left = Math.max(0, r.left),
      top = Math.max(0, r.top);
    const width = Math.min(window.innerWidth, r.right) - left,
      height = Math.min(window.innerHeight, r.bottom) - top;
    if (width <= 0 || height <= 0) return;
    const key = [left, top, width, height].map((n) => n.toFixed(1)).join(':');
    if (key === lastBounds.current) return;
    lastBounds.current = key;
    void ipc
      .reportHitRegion({ x: left, y: top, width, height })
      .catch((e) => notify(String(e), true));
  };
  useLayoutEffect(() => {
    report();
    const observer = new ResizeObserver(report);
    if (shell.current) observer.observe(shell.current);
    window.addEventListener('resize', report);
    return () => {
      observer.disconnect();
      window.removeEventListener('resize', report);
    };
  }, [slot, expanded, peek, dock, dropping]);
  const trackAnimation = () => {
    if (hitFrame.current) cancelAnimationFrame(hitFrame.current);
    const started = performance.now();
    const frame = () => {
      report();
      if (performance.now() - started < 700) hitFrame.current = requestAnimationFrame(frame);
      else hitFrame.current = null;
    };
    hitFrame.current = requestAnimationFrame(frame);
  };
  useEffect(
    () => () => {
      if (hitFrame.current) cancelAnimationFrame(hitFrame.current);
    },
    [],
  );
  const pillWidth = slot?.pill_width ?? 148;
  const width = expanded
      ? 432
      : dropping
        ? Math.max(212, pillWidth + 64)
        : dock
          ? 324
          : peek
            ? 292
            : pillWidth,
    height = expanded ? 324 : dropping ? 46 : dock ? 64 : peek ? 48 : 36;
  const frameWidth = slot ? 560 : window.innerWidth,
    frameHeight = slot ? 360 : window.innerHeight,
    anchorX = slot?.anchor_x ?? frameWidth / 2,
    anchorY = slot?.anchor_y ?? frameHeight - 30;
  const left = Math.min(frameWidth - width - 8, Math.max(8, anchorX - width / 2)),
    top = Math.min(
      frameHeight - height - 6,
      Math.max(6, anchorY < 170 ? anchorY - 18 : anchorY + 18 - height),
    );
  const activeTimer = data?.timer.status === 'running' || data?.timer.status === 'paused',
    title = island.tab === 'settings' ? 'Settings' : tabs.find((t) => t.id === island.tab)?.label;
  const theme =
    data?.settings.theme === 'system' || !data?.settings.theme
      ? systemLight
        ? 'light'
        : 'dark'
      : data.settings.theme;
  return (
    <main
      className="island-canvas select-none"
      data-theme={theme}
      data-motion-paused={paused || reduced}
    >
      <motion.div
        ref={shell}
        layout
        className={`island-pill island-shell ${expanded ? 'is-expanded' : dock ? 'is-dock' : peek ? 'is-peek' : ''} ${island.dropHover ? 'is-drop-target' : ''} ${docked && !privateMode && !expanded && !dock && !peek ? 'has-dock' : ''}`}
        initial={false}
        style={{
          position: 'absolute',
          left,
          top,
          width,
          height,
          borderRadius: expanded ? 28 : dock ? 24 : 999,
        }}
        transition={
          reduced
            ? { duration: 0.1 }
            : { layout: { type: 'spring', stiffness: 380, damping: 30, mass: 0.9 } }
        }
        onLayoutAnimationStart={trackAnimation}
        onLayoutAnimationComplete={report}
        role={expanded || dock ? 'region' : 'button'}
        tabIndex={expanded || dock ? -1 : 0}
        aria-label={
          privateMode
            ? 'Isle presentation mode'
            : expanded
              ? `Isle ${title}`
              : dock
                ? 'Isle dock'
                : 'Open Isle'
        }
        onPointerEnter={enter}
        onPointerLeave={exit}
        onPointerDownCapture={(event) => {
          if (
            expanded &&
            nativeAvailable &&
            !(event.target instanceof Element && event.target.closest('[data-no-focus]'))
          )
            void ipc.setFocusable(true).catch((e) => notify(String(e), true));
        }}
        onClick={(event) => {
          if (!expanded && !dock && !event.altKey) open(island.notice?.tab, true);
        }}
        onContextMenu={(event) => {
          if (!expanded) {
            event.preventDefault();
            if (nativeAvailable) void task(ipc.openPositionMenu);
          }
        }}
        onPointerDown={(event) => {
          if (!expanded && event.altKey && event.button === 0 && nativeAvailable) {
            event.preventDefault();
            void task(ipc.startCustomDrag);
          }
        }}
        onKeyDown={(event) => {
          if (event.key === 'Escape') {
            event.preventDefault();
            close();
          } else if (!expanded && ['Enter', ' '].includes(event.key)) {
            event.preventDefault();
            open(undefined, true);
          } else if (expanded && event.key === 'Tab') {
            const elements = Array.from(
              shell.current?.querySelectorAll<HTMLElement>(
                'button:not(:disabled),input:not(:disabled),textarea,select,[tabindex="0"]',
              ) ?? [],
            ).filter((e) => e.offsetParent !== null);
            if (elements.length) {
              if (event.shiftKey && document.activeElement === elements[0]) {
                event.preventDefault();
                elements.at(-1)?.focus();
              } else if (!event.shiftKey && document.activeElement === elements.at(-1)) {
                event.preventDefault();
                elements[0].focus();
              }
            }
          }
        }}
        onPaste={(event) => {
          if (
            expanded &&
            island.tab === 'shelf' &&
            !(event.target instanceof Element && event.target.matches('input,textarea'))
          ) {
            event.preventDefault();
            void task(api.paste).then(() => refreshShelf());
          }
        }}
      >
        <Ambient quiet={privateMode} expanded={expanded} />
        <AnimatePresence initial={false} mode="wait">
          {dropping ? (
            <DockDrop key="drop" reduced={reduced} />
          ) : dock ? (
            <DockTray key="dock" reduced={reduced} open={open} />
          ) : !expanded ? (
            <IdlePill
              key="idle"
              reduced={reduced}
              paused={paused}
              engaged={engaged}
              peek={peek}
              privateMode={privateMode}
              activeTimer={activeTimer}
              close={close}
            />
          ) : (
            <Panel
              key="panel"
              reduced={reduced}
              privateMode={privateMode}
              activeTimer={activeTimer}
              title={title}
              open={open}
              close={close}
            />
          )}
        </AnimatePresence>
        {docked > 0 && !privateMode && !expanded && !dock && !peek && !dropping && (
          <DockBadge count={docked} />
        )}
      </motion.div>
    </main>
  );
}
