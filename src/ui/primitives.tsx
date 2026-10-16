import type { ButtonHTMLAttributes, ReactNode } from 'react';
import { Icon, type IconName } from './Icon';
import { errorMessage } from '../lib/errors';
export function IconButton({
  icon,
  label,
  active = false,
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { icon: IconName; label: string; active?: boolean }) {
  return (
    <button
      type="button"
      className={`icon-button ${active ? 'active' : ''}`}
      aria-label={label}
      title={label}
      {...props}
    >
      <Icon name={icon} />
    </button>
  );
}
export function Empty({
  icon,
  title,
  children,
  action,
  alert = false,
}: {
  icon: IconName;
  title: string;
  children: ReactNode;
  action?: ReactNode;
  alert?: boolean;
}) {
  return (
    <div className="empty-state" role={alert ? 'alert' : undefined}>
      <span className="empty-orbit">
        <Icon name={icon} size={25} />
      </span>
      <h3>{title}</h3>
      <p>{children}</p>
      {action}
    </div>
  );
}
/** The one way a tool says it is waiting. `inline` keeps it to a line of text. */
export function Loading({ label, inline = false }: { label: string; inline?: boolean }) {
  if (inline)
    return (
      <p className="loading-copy" role="status">
        {label}
      </p>
    );
  return (
    <div className="loading-state" role="status">
      <span className="spinner" aria-hidden="true" />
      <p>{label}</p>
    </div>
  );
}
/** The one way a tool says something failed, with a retry when one makes sense. */
export function ErrorState({
  icon,
  title,
  error,
  onRetry,
}: {
  icon: IconName;
  title: string;
  error: unknown;
  onRetry?: () => void;
}) {
  return (
    <Empty
      icon={icon}
      title={title}
      alert
      action={onRetry ? <button onClick={onRetry}>Try again</button> : undefined}
    >
      {errorMessage(error)}
    </Empty>
  );
}
export function ProgressRing({
  value,
  size = 42,
  children,
}: {
  value: number;
  size?: number;
  children?: ReactNode;
}) {
  const radius = 18;
  const circumference = 2 * Math.PI * radius;
  return (
    <span className="progress-ring" style={{ width: size, height: size }}>
      <svg viewBox="0 0 44 44" aria-hidden="true">
        <circle cx="22" cy="22" r={radius} className="ring-track" />
        <circle
          cx="22"
          cy="22"
          r={radius}
          className="ring-value"
          strokeDasharray={circumference}
          strokeDashoffset={circumference * (1 - Math.max(0, Math.min(1, value)))}
        />
      </svg>
      {children && <span className="ring-content">{children}</span>}
    </span>
  );
}
