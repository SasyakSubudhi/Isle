import { useEffect, useState, type ReactNode } from 'react';
import { api } from '../../lib/api';
import type { AiUsage } from '../../lib/types';
import { Empty, ErrorState, Loading } from '../../ui/primitives';
import { Icon } from '../../ui/Icon';
import { ago, formatTokens, resetTime, until, windowName } from './aiFormat';

type Claude = NonNullable<AiUsage['claude']>;
type Codex = NonNullable<AiUsage['codex']>;

function Meter({ value, tone }: { value: number; tone?: 'warn' | 'full' }) {
  return (
    <span className={`ai-meter ${tone ? `is-${tone}` : ''}`} aria-hidden="true">
      <span style={{ width: `${Math.max(0, Math.min(100, value * 100))}%` }} />
    </span>
  );
}

function ClaudeCard({ claude, now, action }: { claude: Claude; now: number; action?: ReactNode }) {
  const { window: w, limit } = claude;
  const blocked = limit?.rejected ? limit : null;
  const peak = Math.max(1, ...claude.days);
  return (
    <article className="widget-card ai-card">
      <header>
        <h4>Claude</h4>
        <span className="ai-source">Claude Code</span>
        {action}
      </header>
      {blocked ? (
        <div className="ai-headline">
          <strong className="is-full">Limit reached</strong>
          <small>
            Resets {resetTime(blocked.resets_at, now)} ({until(blocked.resets_at, now)})
          </small>
        </div>
      ) : w ? (
        <>
          <div className="ai-headline">
            <strong>{formatTokens(w.tokens)} tokens</strong>
            <small>
              in this 5-hour window, {w.messages} {w.messages === 1 ? 'reply' : 'replies'}
            </small>
          </div>
          {/* Time through the window: Claude doesn't say how much is left. */}
          <Meter value={(now - w.start) / (w.end - w.start)} />

          <p className="ai-line">
            {w.exact_reset ? 'Resets' : 'Resets about'} {resetTime(w.end, now)} ({until(w.end, now)}
            ){w.exact_reset ? null : <span className="ai-tag">estimate</span>}
          </p>
        </>
      ) : (
        <p className="widget-hint">
          No Claude Code use in the last 5 hours, so your limit is fresh. A new window starts with
          your next message.
        </p>
      )}
      <div className="ai-days" aria-label="Tokens per day, last 7 days">
        {claude.days.map((n, i) => (
          <span key={i} title={`${formatTokens(n)} tokens`}>
            <span style={{ height: `${Math.max(n ? 8 : 0, (n / peak) * 100)}%` }} />
          </span>
        ))}
      </div>
      <p className="ai-line">
        Today {formatTokens(claude.today_tokens)} · 7 days {formatTokens(claude.week_tokens)}
        {claude.last_used ? ` · last used ${ago(claude.last_used, now)}` : ''}
      </p>
      <p className="widget-hint ai-note">
        Counted from Claude Code. Claude doesn’t share how much is left, and app or website chats
        aren’t included.
      </p>
    </article>
  );
}

function CodexCard({ codex, now, action }: { codex: Codex; now: number; action?: ReactNode }) {
  // Codex only writes its limits here during a session on this PC. Use on the
  // web, in the cloud or on another device moves the real numbers on unseen.
  const stale = now - codex.updated_at > 30 * 60_000;
  return (
    <article className="widget-card ai-card">
      <header>
        <h4>ChatGPT</h4>
        <span className="ai-source">
          Codex{codex.plan ? ` · ${codex.plan[0].toUpperCase()}${codex.plan.slice(1)}` : ''}
        </span>
        {action}
      </header>
      {codex.windows.map((w) => {
        const done = w.resets_at <= now;
        const used = done ? 0 : w.used_percent;
        return (
          <div className="ai-window" key={w.minutes}>
            <p className="ai-line">
              <span>{windowName(w.minutes)} limit</span>
              <span className="spacer" />
              <strong>
                {done && stale
                  ? 'Reset'
                  : `${stale ? 'At most ' : ''}${Math.round(100 - used)}% left`}
              </strong>
            </p>
            <Meter
              value={used / 100}
              tone={used >= 100 ? 'full' : used >= 80 ? 'warn' : undefined}
            />
            <p className="ai-line ai-sub">
              {done
                ? stale
                  ? `Reset ${resetTime(w.resets_at, now)}; use since then isn’t known yet`
                  : 'Reset, fully available'
                : `${Math.round(used)}% used · resets ${resetTime(w.resets_at, now)} (${until(w.resets_at, now)})`}
            </p>
          </div>
        );
      })}
      <p className="widget-hint ai-note">
        {stale
          ? `Last reported by Codex on this PC ${ago(codex.updated_at, now)}. Use elsewhere since then (web, cloud tasks, another device) shows after your next Codex session here.`
          : `Exact, from Codex ${ago(codex.updated_at, now)}. Use elsewhere (web, cloud tasks, another device) shows after your next Codex session here.`}
      </p>
    </article>
  );
}

export function AiUsageView() {
  const [usage, setUsage] = useState<AiUsage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [now, setNow] = useState(() => Date.now());
  const load = (fresh: boolean) => {
    setBusy(true);
    void api
      .aiUsage(fresh)
      .then((value) => {
        setUsage(value);
        setError(null);
      })
      .catch((e) => setError(String(e)))
      .finally(() => {
        setBusy(false);
        setNow(Date.now());
      });
  };
  const refresh = (
    <button
      className="icon-button"
      aria-label="Refresh"
      title="Refresh"
      disabled={busy}
      onClick={() => load(true)}
    >
      <Icon name="refresh" size={14} />
    </button>
  );
  useEffect(() => {
    load(false);
    const stop = api.on<AiUsage>('ai://changed', setUsage);
    const tick = setInterval(() => setNow(Date.now()), 30_000);
    return () => {
      clearInterval(tick);
      void stop.then((unlisten) => unlisten()).catch(() => {});
    };
  }, []);
  return (
    <section className="feature widgets-view ai-view">
      {error ? (
        <ErrorState
          icon="ai"
          title="AI usage couldn’t load"
          error={error}
          onRetry={() => load(true)}
        />
      ) : !usage ? (
        <Loading inline label="Reading your AI tool logs…" />
      ) : !usage.claude && !usage.codex ? (
        <Empty icon="ai" title="No AI tools found">
          Isle reads limits from Claude Code and Codex on this PC. Use either one and they’ll show
          up here.
        </Empty>
      ) : (
        <>
          {usage.claude ? <ClaudeCard claude={usage.claude} now={now} action={refresh} /> : null}
          {usage.codex ? (
            <CodexCard codex={usage.codex} now={now} action={usage.claude ? undefined : refresh} />
          ) : null}
        </>
      )}
    </section>
  );
}
