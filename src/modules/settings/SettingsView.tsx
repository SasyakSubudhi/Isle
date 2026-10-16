import { useEffect, useState } from 'react';
import { api } from '../../lib/api';
import { ipc, type WindowPreferences } from '../../lib/ipc';
import { task, useApp } from '../../app/store';
import type { FeatureSettings } from '../../lib/types';
import { Icon } from '../../ui/Icon';
import { tabs } from '../../island/islandTabs';
import { changedFields, clampNumber, hotkeyFromEvent, rebaseDraft } from '../../lib/settingsDraft';
function Toggle({
  label,
  hint,
  checked,
  onChange,
}: {
  label: string;
  hint?: string;
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <label className="setting-toggle">
      <span>
        <strong>{label}</strong>
        {hint && <small>{hint}</small>}
      </span>
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
    </label>
  );
}
/** Free typing, but only a value inside [min,max] ever leaves the field. */
function NumberField({
  value,
  min,
  max,
  onCommit,
}: {
  value: number;
  min: number;
  max: number;
  onCommit: (value: number) => void;
}) {
  const [text, setText] = useState(String(value));
  useEffect(() => setText(String(value)), [value]);
  return (
    <input
      type="number"
      min={min}
      max={max}
      value={text}
      onChange={(e) => {
        setText(e.target.value);
        const n = Number(e.target.value);
        if (e.target.value.trim() !== '' && Number.isFinite(n) && n >= min && n <= max)
          onCommit(Math.round(n));
      }}
      onBlur={() => {
        const n = clampNumber(text, min, max, value);
        setText(String(n));
        if (n !== value) onCommit(n);
      }}
    />
  );
}
/** Click, then press the combination you want. Escape cancels. */
function HotkeyField({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  const [recording, setRecording] = useState(false);
  return (
    <input
      readOnly
      className={recording ? 'hotkey-recording' : undefined}
      value={recording ? 'Press a key combination…' : value}
      onFocus={() => setRecording(true)}
      onBlur={() => setRecording(false)}
      onKeyDown={(e) => {
        if (e.key === 'Tab') return;
        e.preventDefault();
        if (e.key === 'Escape') {
          e.currentTarget.blur();
          return;
        }
        const combo = hotkeyFromEvent(e);
        if (combo) {
          onChange(combo);
          e.currentTarget.blur();
        }
      }}
    />
  );
}
export function SettingsView() {
  const data = useApp((s) => s.data)!;
  const patch = useApp((s) => s.patch);
  const [draft, setDraft] = useState(data.settings);
  const [base, setBase] = useState(data.settings);
  const [position, setPosition] = useState<WindowPreferences | null>(null);
  const [busy, setBusy] = useState(false);
  const [clear, setClear] = useState(false);
  const dirty = Object.keys(changedFields(base, draft)).length > 0;
  useEffect(() => {
    void ipc
      .positionGet()
      .then(setPosition)
      .catch((e) => useApp.getState().notify(String(e), true));
  }, []);
  // Settings changed from the tray or another button: take them in, keep unsaved edits.
  useEffect(() => {
    if (data.settings !== base) {
      setDraft((d) => rebaseDraft(base, d, data.settings));
      setBase(data.settings);
    }
  }, [data.settings]); // eslint-disable-line react-hooks/exhaustive-deps
  const field = <K extends keyof FeatureSettings>(key: K, value: FeatureSettings[K]) =>
    setDraft((d) => ({ ...d, [key]: value }));
  const save = async () => {
    setBusy(true);
    try {
      const settings = await api.settingsPatch(changedFields(base, draft));
      setBase(settings);
      setDraft(settings);
      patch({ settings });
      useApp.getState().notify('Settings saved');
    } catch (e) {
      useApp.getState().notify(String(e), true);
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="feature settings-view">
      <div className="toolbar sticky-toolbar">
        <h3>Make Isle yours</h3>
        <span className="spacer" />
        <button
          className="primary-button compact-button"
          disabled={busy || !dirty}
          onClick={() => void save()}
        >
          {busy ? 'Saving…' : 'Save changes'}
        </button>
      </div>
      <div className="settings-group">
        <h4>Chrome shortcuts</h4>
        <p>
          Choose the Chrome profile that already has your Gmail, YouTube and WhatsApp accounts
          signed in.
        </p>
        <label>
          Profile
          <select
            value={draft.chrome_profile}
            onChange={(e) => field('chrome_profile', e.target.value)}
          >
            <option value="">Choose your signed-in profile</option>
            {data.chrome.profiles.map((profile) => (
              <option key={profile.directory} value={profile.directory}>
                {profile.name}
                {profile.account ? ` · ${profile.account}` : ''}
              </option>
            ))}
          </select>
        </label>
        {data.chrome.profiles.length === 0 && (
          <p className="inline-error">
            No Chrome profiles found. Open Chrome and create or sign in to a profile first.
          </p>
        )}
        <label>
          Chrome executable
          <input
            value={draft.chrome_executable}
            onChange={(e) => field('chrome_executable', e.target.value)}
            placeholder={
              data.chrome.executable
                ? `Detected: ${data.chrome.executable}`
                : 'Full path to chrome.exe'
            }
          />
          <small>Leave empty to use the detected Chrome. Only chrome.exe is allowed.</small>
        </label>
        {(['gmail_url', 'youtube_url', 'whatsapp_url'] as const).map((key) => (
          <label key={key}>
            {key === 'gmail_url' ? 'Gmail' : key === 'youtube_url' ? 'YouTube' : 'WhatsApp'}
            <input value={draft[key]} onChange={(e) => field(key, e.target.value)} />
          </label>
        ))}
      </div>
      <div className="settings-group">
        <h4>Appearance & position</h4>
        <label>
          Theme
          <select
            value={draft.theme}
            onChange={(e) => field('theme', e.target.value as FeatureSettings['theme'])}
          >
            <option value="system">Follow Windows</option>
            <option value="dark">Dark</option>
            <option value="light">Light</option>
          </select>
        </label>
        <Toggle
          label="Reduced motion"
          hint="Keep ambient motion still and use short fades."
          checked={draft.reduced_motion}
          onChange={(v) => field('reduced_motion', v)}
        />
        <Toggle
          label="Expand on hover"
          checked={draft.hover_expand}
          onChange={(v) => field('hover_expand', v)}
        />
        {position && (
          <>
            <label>
              Position
              <select
                value={position.position_mode}
                onChange={(e) => {
                  const next = {
                    ...position,
                    position_mode: e.target.value as WindowPreferences['position_mode'],
                  };
                  void task(() => ipc.positionSet(next)).then(() => setPosition(next));
                }}
              >
                <option value="taskbar">Taskbar free space</option>
                <option value="top-center">Top-center</option>
                <option value="custom">Custom · Alt-drag pill to place</option>
              </select>
            </label>
            <label>
              Pill width
              <NumberField
                min={120}
                max={240}
                value={position.pill_width}
                onCommit={(pill_width) => {
                  const next = { ...position, pill_width };
                  setPosition(next);
                  void task(() => ipc.positionSet(next));
                }}
              />
            </label>
          </>
        )}
        <Toggle
          label="Presentation mode"
          hint="Hide names, previews and notifications. Tracking and timers continue."
          checked={draft.presentation_mode}
          onChange={(v) => field('presentation_mode', v)}
        />
      </div>
      <div className="settings-group">
        <h4>Tab bar</h4>
        <small>Untick the tools you don't use to keep the tab bar roomy. Shelf always stays.</small>
        <div className="tab-picker">
          {tabs
            .filter((tab) => tab.id !== 'shelf')
            .map((tab) => {
              const shown = !(draft.hidden_tabs ?? []).includes(tab.id);
              return (
                <label key={tab.id} className={shown ? 'is-on' : ''}>
                  <input
                    type="checkbox"
                    checked={shown}
                    onChange={(e) =>
                      field(
                        'hidden_tabs',
                        e.target.checked
                          ? (draft.hidden_tabs ?? []).filter((id) => id !== tab.id)
                          : [...(draft.hidden_tabs ?? []), tab.id],
                      )
                    }
                  />
                  <Icon name={tab.icon} size={14} />
                  <span>{tab.label}</span>
                </label>
              );
            })}
        </div>
      </div>
      <div className="settings-group">
        <h4>Pill & widgets</h4>
        <label>
          Pill style
          <select
            value={draft.pill_theme}
            onChange={(e) => field('pill_theme', e.target.value as FeatureSettings['pill_theme'])}
          >
            <option value="tide">Tide · monochrome waves</option>
            <option value="still">Still · no motion, just the wordmark</option>
            <option value="evergreen">Evergreen · tide waves in deep pine green</option>
            <option value="outline">Outline · hairline glass</option>
          </select>
        </label>
        <Toggle
          label="Context-aware pill"
          hint="Shows a timer for coding, meetings, reading or writing. Isle looks only at the app's name, never window titles or content."
          checked={draft.context_pill}
          onChange={(v) => field('context_pill', v)}
        />
        <Toggle
          label="Weather widget"
          hint="Uses the internet: sends the city below to open-meteo.com, at most every 30 minutes."
          checked={draft.weather_enabled}
          onChange={(v) => field('weather_enabled', v)}
        />
        {draft.weather_enabled && (
          <>
            <label>
              City
              <input
                value={draft.weather_place}
                maxLength={80}
                placeholder="e.g. Bengaluru, or 12.97, 77.59"
                onChange={(e) => field('weather_place', e.target.value)}
              />
            </label>
            <Toggle
              label="Fahrenheit"
              checked={draft.weather_fahrenheit}
              onChange={(v) => field('weather_fahrenheit', v)}
            />
          </>
        )}
      </div>
      <div className="settings-group">
        <h4>Clipboard & Shelf</h4>
        <Toggle
          label="Clipboard history"
          hint="Opt in to capture future text, images and files locally."
          checked={draft.clipboard_history}
          onChange={(v) => field('clipboard_history', v)}
        />
        <Toggle
          label="Copy files into Shelf cache"
          hint="Cached copies survive when the original is removed."
          checked={draft.copy_into_cache}
          onChange={(v) => field('copy_into_cache', v)}
        />
        <label>
          Expire unpinned items after (hours)
          <NumberField
            min={1}
            max={720}
            value={draft.shelf_expiry_hours}
            onCommit={(v) => field('shelf_expiry_hours', v)}
          />
        </label>
        <Toggle
          label="Watch Downloads"
          checked={draft.watch_downloads}
          onChange={(v) => field('watch_downloads', v)}
        />
        <label>
          Downloads folder
          <input
            value={draft.downloads_folder}
            onChange={(e) => field('downloads_folder', e.target.value)}
          />
        </label>
        <Toggle
          label="Automatically add finished downloads"
          hint="Otherwise Isle asks before adding each file."
          checked={draft.auto_add_downloads}
          onChange={(v) => field('auto_add_downloads', v)}
        />
      </div>
      <div className="settings-group">
        <h4>Focus timer</h4>
        <div className="duration-fields">
          {(
            [
              ['focus_minutes', 'Focus'],
              ['short_break_minutes', 'Short break'],
              ['long_break_minutes', 'Long break'],
            ] as const
          ).map(([key, label]) => (
            <label key={key}>
              {label}
              <NumberField min={1} max={180} value={draft[key]} onCommit={(v) => field(key, v)} />
              <small>minutes</small>
            </label>
          ))}
        </div>
      </div>
      <div className="settings-group">
        <h4>Screen time</h4>
        <Toggle
          label="Pause tracking"
          checked={draft.tracking_paused}
          onChange={(v) => field('tracking_paused', v)}
        />
        <label>
          Stop counting after idle seconds
          <NumberField
            min={10}
            max={3600}
            value={draft.idle_timeout_seconds}
            onCommit={(v) => field('idle_timeout_seconds', v)}
          />
        </label>
        <Toggle
          label="Store window titles"
          hint="Off by default for privacy. Titles stay on this computer."
          checked={draft.track_window_titles}
          onChange={(v) => field('track_window_titles', v)}
        />
        {draft.excluded_apps.length > 0 && (
          <button className="text-button" onClick={() => field('excluded_apps', [])}>
            Restore {draft.excluded_apps.length} excluded{' '}
            {draft.excluded_apps.length === 1 ? 'app' : 'apps'}
          </button>
        )}
        {clear ? (
          <div className="danger-confirm">
            <p>Delete all recorded screen-time sessions? Shelf and notes stay.</p>
            <button
              className="danger"
              onClick={() =>
                void task(api.clearUsage, 'Screen-time data cleared').then(() => setClear(false))
              }
            >
              Delete screen-time data
            </button>
            <button onClick={() => setClear(false)}>Cancel</button>
          </div>
        ) : (
          <button className="text-button danger" onClick={() => setClear(true)}>
            <Icon name="trash" size={14} />
            Clear screen-time data…
          </button>
        )}
      </div>
      <div className="settings-group">
        <h4>Hotkeys & startup</h4>
        {Object.entries(draft.hotkeys).map(([name, value]) => (
          <label key={name}>
            {name === 'selection'
              ? 'Look up selected text'
              : name === 'toggle'
                ? 'Toggle Isle'
                : name === 'usage'
                  ? 'Screen time'
                  : name[0].toUpperCase() + name.slice(1)}
            <HotkeyField
              value={value}
              onChange={(combo) => field('hotkeys', { ...draft.hotkeys, [name]: combo })}
            />
          </label>
        ))}
        <Toggle
          label="Launch at Windows startup"
          checked={draft.launch_at_startup}
          onChange={(v) => field('launch_at_startup', v)}
        />
        {data.hotkey_errors.length > 0 && (
          <p className="inline-error">{data.hotkey_errors.join('\n')}</p>
        )}
      </div>
      <div className="settings-group">
        <button className="text-button danger" onClick={() => void task(api.quit)}>
          <Icon name="power" size={14} />
          Quit Isle
        </button>
      </div>
      <p className="local-note">
        <Icon name="privacy" size={13} />
        Stored on this computer. No telemetry or cloud sync. Dictionary lookups, including Look up
        selected text, send the word to dictionaryapi.dev. The weather widget, when on, sends your
        city to open-meteo.com.
      </p>
    </section>
  );
}
