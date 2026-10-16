import { AnimatePresence, motion } from 'framer-motion';
import { DUR, EASE_OUT } from '../lib/motion';
import { nativeAvailable } from '../lib/ipc';
import { api, formatClock } from '../lib/api';
import type { Tab } from '../lib/types';
import { task, useApp } from '../app/store';
import { IsleMark } from '../ui/IsleMark';
import { Icon } from '../ui/Icon';
import { Empty, ErrorState, IconButton, Loading } from '../ui/primitives';
import { tabs, views } from './islandTabs';

/** The expanded panel: header, tool tabs, the active tool, footer and toasts. */
export function Panel({
  reduced,
  privateMode,
  activeTimer,
  title,
  open,
  close,
}: {
  reduced: boolean;
  privateMode: boolean;
  activeTimer: boolean;
  title: string | undefined;
  open: (tab?: Tab, focus?: boolean, pin?: boolean) => void;
  close: () => void;
}) {
  const { island, data, loading, bootError, toast, capturing, dispatch, patch, notify, load } =
    useApp();
  const View = views[island.tab];
  const launch = (service: string) => {
    if (!data?.settings.chrome_profile) {
      open('settings', true, true);
      notify('Choose your signed-in Chrome profile to open your accounts.');
      return;
    }
    void task(() => api.chrome(service));
  };
  return (
    <motion.div
      key="panel"
      className="panel-content"
      initial={{ opacity: 0, y: reduced ? 0 : 5 }}
      animate={{
        opacity: 1,
        y: 0,
        transition: { delay: reduced ? 0 : 0.1, duration: DUR.base },
      }}
      exit={{ opacity: 0, transition: { duration: DUR.quick } }}
    >
      <header className="panel-header">
        <IsleMark state="idle" engaged={false} />
        <span className="panel-name">isle</span>
        <span className="header-divider" />
        <span className="panel-title">{privateMode ? 'Presentation mode' : title}</span>
        <span className="spacer" />
        {!privateMode && (
          <IconButton
            icon="settings"
            label="Settings"
            active={island.tab === 'settings'}
            onClick={() => open('settings')}
          />
        )}
        <IconButton
          icon="pin"
          label={island.pinned ? 'Unpin panel' : 'Keep panel open'}
          active={island.pinned}
          onClick={() => dispatch({ type: 'pin' })}
        />
        <IconButton icon="close" label="Collapse Isle" onClick={close} />
        <IconButton icon="power" label="Quit Isle" onClick={() => void task(api.quit)} />
      </header>
      {privateMode ? (
        <div className="privacy-panel">
          <Empty
            icon="privacy"
            title="Your space stays private"
            action={
              <button
                className="primary-button"
                onClick={() =>
                  void task(() => api.settingsPatch({ presentation_mode: false })).then(
                    (settings) => {
                      if (settings) patch({ settings });
                    },
                  )
                }
              >
                Exit presentation mode
              </button>
            }
          >
            Previews and notifications are hidden. Tracking and timers continue.
          </Empty>
          {activeTimer && data && (
            <span className="privacy-timer">
              {formatClock(data.timer.remaining_seconds)} remaining
            </span>
          )}
        </div>
      ) : (
        <>
          <nav className="tab-bar" aria-label="Isle tools">
            {tabs
              .filter(
                (tab) => tab.id === island.tab || !data?.settings.hidden_tabs?.includes(tab.id),
              )
              .map((tab) => (
                <button
                  type="button"
                  key={tab.id}
                  aria-label={tab.label}
                  aria-pressed={island.tab === tab.id}
                  title={tab.label}
                  onClick={() => open(tab.id)}
                >
                  {island.tab === tab.id && (
                    <motion.span
                      layoutId="active-tab"
                      className="tab-highlight"
                      transition={
                        reduced ? { duration: 0 } : { type: 'spring', stiffness: 400, damping: 32 }
                      }
                    />
                  )}
                  <Icon name={tab.icon} />
                </button>
              ))}
          </nav>
          {island.notice?.kind === 'limit' && (
            <div className="limit-notice" role="status">
              <span>{island.notice.title}</span>
              <button
                onClick={() =>
                  void task(() => api.snooze(island.notice!.appId!), 'Snoozed for 15 minutes').then(
                    (ok) => {
                      if (ok) useApp.setState((s) => ({ island: { ...s.island, notice: null } }));
                    },
                  )
                }
              >
                Snooze
              </button>
              <button
                onClick={() => {
                  open('focus', true, true);
                  useApp.setState((s) => ({ island: { ...s.island, notice: null } }));
                  void task(() => api.timer('start')).then((timer) => {
                    if (timer) patch({ timer });
                  });
                }}
              >
                Focus
              </button>
              <IconButton
                icon="close"
                label="Dismiss reminder"
                onClick={() => useApp.setState((s) => ({ island: { ...s.island, notice: null } }))}
              />
            </div>
          )}
          <div className="panel-body">
            {loading && !data ? (
              <Loading label="Opening your local space…" />
            ) : bootError ? (
              <ErrorState
                icon="shelf"
                title={nativeAvailable ? 'Your space couldn’t open' : 'Open the Isle desktop app'}
                error={bootError}
                onRetry={() => void load()}
              />
            ) : data ? (
              <motion.div
                key={island.tab}
                className="view-frame"
                initial={reduced ? false : { opacity: 0, y: 6 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: DUR.base, ease: EASE_OUT }}
              >
                <View />
              </motion.div>
            ) : null}
            {island.dropHover && (
              <div className="drop-overlay">
                <Icon name="shelf" size={32} />
                <strong>Drop it here</strong>
                <span>Files stay where they are.</span>
              </div>
            )}
          </div>
          <footer className="panel-footer">
            <div className="quick-launch">
              <button
                data-app="gmail"
                title="Open Gmail in your Chrome profile"
                onClick={() => launch('gmail')}
              >
                <Icon name="gmail" size={15} />
                <span>Gmail</span>
              </button>
              <button
                data-app="youtube"
                title="Open YouTube in your Chrome profile"
                onClick={() => launch('youtube')}
              >
                <Icon name="youtube" size={15} />
                <span>YouTube</span>
              </button>
              <button
                data-app="whatsapp"
                title="Open WhatsApp in your Chrome profile"
                onClick={() => launch('whatsapp')}
              >
                <Icon name="whatsapp" size={15} />
                <span>WhatsApp</span>
              </button>
            </div>
            <span className="spacer" />
            <IconButton
              icon="privacy"
              label="Enable presentation mode"
              onClick={() => {
                if (data)
                  void task(() => api.settingsPatch({ presentation_mode: true })).then(
                    (settings) => {
                      if (settings) patch({ settings });
                    },
                  );
              }}
            />
          </footer>
        </>
      )}
      {capturing && (
        <div className="capture-status" role="status">
          Finish your screen snip. It will arrive on Shelf.
        </div>
      )}
      <AnimatePresence>
        {toast && !privateMode && (
          <motion.div
            key={toast.message}
            className={`toast ${toast.error ? 'error' : ''}`}
            role={toast.error ? 'alert' : 'status'}
            initial={reduced ? { opacity: 0 } : { opacity: 0, y: 10, scale: 0.98 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: reduced ? 0 : 6, transition: { duration: DUR.quick } }}
            transition={
              reduced ? { duration: DUR.quick } : { type: 'spring', stiffness: 520, damping: 34 }
            }
          >
            <Icon name={toast.error ? 'close' : 'check'} size={14} />
            {toast.message}
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}
