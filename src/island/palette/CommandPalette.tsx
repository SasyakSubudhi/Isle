import { useEffect, useMemo, useRef, useState } from 'react';
import { create } from 'zustand';
import { api } from '../../lib/api';
import { rank } from '../../lib/rank';
import { dndActive } from '../../lib/dnd';
import type { Tab } from '../../lib/types';
import { task, useApp } from '../../app/store';
import { Icon, type IconName } from '../../ui/Icon';
import { tabs } from '../islandTabs';
import { useDock } from '../../modules/dock/dock';
import { useLive } from '../live/useLiveActivity';

export const usePalette = create<{ open: boolean }>(() => ({ open: false }));
export const openPalette = () => usePalette.setState({ open: true });
const closePalette = () => usePalette.setState({ open: false });

interface Command {
  id: string;
  label: string;
  hint: string;
  icon: IconName;
  run: () => void;
}

/** One search box for every tool, action, Shelf item and docked file. */
export function CommandPalette({ open }: { open: (tab?: Tab) => void }) {
  const { data, notify } = useApp();
  const docked = useDock((s) => s.items);
  const activity = useLive((s) => s.activity);
  const [query, setQuery] = useState('');
  const [active, setActive] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => input.current?.focus(), []);

  const commands = useMemo<Command[]>(() => {
    const settings = data?.settings;
    const dnd = dndActive(settings, activity);
    const patch = (change: Parameters<typeof api.settingsPatch>[0], message: string) =>
      void task(() => api.settingsPatch(change), message).then(
        (s) => s && useApp.getState().patch({ settings: s }),
      );
    const go: Command[] = [
      ...tabs.map((t) => ({
        id: `tab:${t.id}`,
        label: t.label,
        hint: 'Open',
        icon: t.icon,
        run: () => open(t.id),
      })),
      {
        id: 'tab:settings',
        label: 'Settings',
        hint: 'Open',
        icon: 'settings',
        run: () => open('settings'),
      },
    ];
    const actions: Command[] = [
      {
        id: 'dnd',
        label: settings?.do_not_disturb ? 'Turn off Do Not Disturb' : 'Turn on Do Not Disturb',
        hint: dnd && !settings?.do_not_disturb ? 'On during your call' : 'Action',
        icon: 'moon',
        run: () =>
          patch(
            { do_not_disturb: !settings?.do_not_disturb },
            settings?.do_not_disturb ? 'Do Not Disturb is off' : 'Do Not Disturb is on',
          ),
      },
      {
        id: 'presentation',
        label: settings?.presentation_mode ? 'Leave presentation mode' : 'Start presentation mode',
        hint: 'Action',
        icon: 'privacy',
        run: () =>
          patch({ presentation_mode: !settings?.presentation_mode }, 'Presentation mode updated'),
      },
      {
        id: 'focus',
        label: data?.timer.status === 'running' ? 'Pause focus timer' : 'Start focus timer',
        hint: 'Action',
        icon: 'focus',
        run: () =>
          void task(() => api.timer(data?.timer.status === 'running' ? 'pause' : 'start')).then(
            (timer) => timer && useApp.getState().patch({ timer }),
          ),
      },
      {
        id: 'capture',
        label: 'Take a screenshot',
        hint: 'Action',
        icon: 'capture',
        run: () => void task(api.capture),
      },
      {
        id: 'paste',
        label: 'Paste clipboard to Shelf',
        hint: 'Action',
        icon: 'copy',
        run: () => void task(api.paste),
      },
      {
        id: 'backup',
        label: 'Back up now',
        hint: 'Action',
        icon: 'backup',
        run: () =>
          void task(api.backupCreate).then(
            (b) => b && notify(`Saved ${b.name} in Documents\\Isle Backups`),
          ),
      },
      {
        id: 'update',
        label: 'Check for updates',
        hint: 'Action',
        icon: 'refresh',
        run: () =>
          void task(api.updateCheck).then(
            (u) =>
              u &&
              notify(
                u.available
                  ? `Isle ${u.available} is available. Install it from Settings.`
                  : `Isle ${u.current} is up to date`,
              ),
          ),
      },
      {
        id: 'quit',
        label: 'Quit Isle',
        hint: 'Action',
        icon: 'power',
        run: () => void task(api.quit),
      },
    ];
    const shelf: Command[] = (data?.shelf ?? [])
      .filter((item) => !item.history)
      .map((item) => ({
        id: `shelf:${item.id}`,
        label: item.name,
        hint: 'Shelf',
        icon: item.kind === 'link' ? 'link' : item.kind === 'image' ? 'image' : 'file',
        run: () =>
          item.kind === 'text'
            ? void task(() => api.copy([item.id]), 'Copied')
            : void task(() => api.open(item.id)),
      }));
    const dock: Command[] = docked.map((item) => ({
      id: `dock:${item.id}`,
      label: item.name,
      hint: 'Dock',
      icon: 'dock',
      run: () => void task(() => api.open(item.id)),
    }));
    return [...go, ...actions, ...dock, ...shelf];
  }, [data, docked, activity, open, notify]);

  const results = rank(commands, query, (c) => `${c.label} ${c.hint}`);
  const selected = Math.min(active, Math.max(0, results.length - 1));
  const run = (command: Command | undefined) => {
    if (!command) return;
    closePalette();
    command.run();
  };
  return (
    <div className="palette" role="dialog" aria-label="Search Isle">
      <label className="palette-field">
        <Icon name="search" size={15} />
        <input
          ref={input}
          value={query}
          placeholder="Search tools, actions and files"
          aria-controls="palette-results"
          aria-activedescendant={results[selected] ? `palette-${selected}` : undefined}
          onChange={(e) => {
            setQuery(e.target.value);
            setActive(0);
          }}
          onKeyDown={(e) => {
            if (e.key === 'ArrowDown') {
              e.preventDefault();
              setActive((selected + 1) % Math.max(1, results.length));
            } else if (e.key === 'ArrowUp') {
              e.preventDefault();
              setActive((selected - 1 + results.length) % Math.max(1, results.length));
            } else if (e.key === 'Enter') {
              e.preventDefault();
              run(results[selected]);
            } else if (e.key === 'Escape') {
              e.preventDefault();
              e.stopPropagation();
              closePalette();
            }
          }}
        />
        <kbd>Esc</kbd>
      </label>
      <ul id="palette-results" className="palette-results" role="listbox">
        {results.map((command, i) => (
          <li
            key={command.id}
            id={`palette-${i}`}
            role="option"
            aria-selected={i === selected}
            className={i === selected ? 'is-active' : undefined}
            onMouseMove={() => setActive(i)}
            onClick={() => run(command)}
          >
            <Icon name={command.icon} size={15} />
            <span>{command.label}</span>
            <small>{command.hint}</small>
          </li>
        ))}
        {!results.length && <li className="palette-empty">Nothing matches “{query}”.</li>}
      </ul>
    </div>
  );
}
