'use client';
/**
 * Opbox Design System — core primitives (ui.tsx)
 * Copy-paste into your Next.js app. Requires kit/tokens.css + kit/opbox.css
 * (or the Tailwind config). Only dependency: clsx (or your cn helper).
 */
import * as React from 'react';
import { clsx } from 'clsx';

const cn = clsx;

/* ── StatusBadge ────────────────────────────────────────────────────────── */
export type Intent = 'success' | 'info' | 'warning' | 'danger' | 'neutral';

/** Maps every lifecycle status in the system to an intent. Extend as needed. */
export const STATUS_TONE: Record<string, Intent> = {
  // success
  completed: 'success', approved: 'success', paid: 'success', live: 'success',
  released: 'success', success: 'success', done: 'success', committed: 'success', active_ok: 'success',
  // info
  open: 'info', running: 'info', leased: 'info', active: 'info', submitted: 'info', authorised: 'info',
  // warning
  pending: 'warning', qa: 'warning', edited: 'warning',
  partially_paid: 'warning', escalated: 'warning', on_hold: 'warning', review: 'warning',
  // danger
  blocked: 'danger', overdue: 'danger', failed: 'danger', rejected: 'danger', voided: 'danger',
  // neutral
  queued: 'neutral', cancelled: 'neutral', backlog: 'neutral', draft: 'neutral', paused: 'neutral',
};
const PULSE = new Set(['running', 'leased']);

export function StatusBadge({ status, label, className }: {
  status: string; label?: string; className?: string;
}) {
  const key = status.toLowerCase();
  const intent = STATUS_TONE[key] ?? 'neutral';
  return (
    <span className={cn('badge', `badge-${intent}`, className)}>
      <span className={cn('dot', PULSE.has(key) && 'pulse')} />
      {label ?? status.replace(/_/g, ' ')}
    </span>
  );
}

/* ── Tag ────────────────────────────────────────────────────────────────── */
export function Tag({ children, flag, className }: { children: React.ReactNode; flag?: boolean; className?: string }) {
  return <span className={cn('tag', flag && 'tag-flag', className)}>{children}</span>;
}

/* ── Button ─────────────────────────────────────────────────────────────── */
export type ButtonVariant = 'primary' | 'success' | 'secondary' | 'ghost' | 'destructive';
export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: 'default' | 'large';
  icon?: React.ReactNode;
  loading?: boolean;
  blockedReason?: string; // criteria-blocked state: disabled + reason
}
export function Button({
  variant = 'secondary', size = 'default', icon, loading, blockedReason,
  disabled, children, className, ...rest
}: ButtonProps) {
  const isDisabled = disabled || loading || !!blockedReason;
  return (
    <span className="inline-flex items-center">
      <button
        className={cn('btn', `btn-${variant}`, size === 'large' && 'btn-lg', className)}
        disabled={isDisabled}
        {...rest}
      >
        {loading ? <Spinner /> : icon}
        {loading ? 'Working…' : children}
      </button>
      {blockedReason && <span className="btn-blocked-note">{blockedReason}</span>}
    </span>
  );
}

export function IconButton({ icon, label, className, ...rest }: {
  icon: React.ReactNode; label: string;
} & React.ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button aria-label={label} title={label} className={cn('icon-btn', className)} {...rest}>
      {icon}
    </button>
  );
}

export function Spinner({ className }: { className?: string }) {
  return (
    <svg className={cn('animate-spin', className)} width="13" height="13" viewBox="0 0 24 24" fill="none">
      <circle cx="12" cy="12" r="10" stroke="currentColor" strokeOpacity=".25" strokeWidth="3" />
      <path d="M22 12a10 10 0 0 0-10-10" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
    </svg>
  );
}

/* ── Panel & PanelHeader ────────────────────────────────────────────────── */
export function Panel({ interactive, selected, className, children, ...rest }: {
  interactive?: boolean; selected?: boolean;
} & React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn('panel', interactive && 'panel-interactive', selected && 'panel-selected', className)}
      {...rest}
    >
      {children}
    </div>
  );
}
export function PanelHeader({ children, right }: { children: React.ReactNode; right?: React.ReactNode }) {
  return <div className="panel-header"><span>{children}</span>{right}</div>;
}

/* ── Input / Field ──────────────────────────────────────────────────────── */
export interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  error?: string;
  mono?: boolean;
}
export function Input({ error, mono, className, ...rest }: InputProps) {
  return (
    <>
      <input className={cn('input', mono && 'mono', error && 'error', className)} {...rest} />
      {error && <div className="input-error-msg">{error}</div>}
    </>
  );
}
export function Field({ label, required, hint, children }: {
  label: string; required?: boolean; hint?: string; children: React.ReactNode;
}) {
  return (
    <div>
      <label className="label">
        {label}{required && <span className="req"> *</span>}
        {hint && <span style={{ color: 'var(--accent)' }}> · {hint}</span>}
      </label>
      {children}
    </div>
  );
}

/* ── Tabs ───────────────────────────────────────────────────────────────── */
export function Tabs({ children }: { children: React.ReactNode }) {
  return <div className="tabs">{children}</div>;
}
export function Tab({ active, count, children, ...rest }: {
  active?: boolean; count?: number;
} & React.ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button className={cn('tab', active && 'active')} {...rest}>
      {children}
      {count != null && <span className="count">{count}</span>}
    </button>
  );
}

/* ── Callout ────────────────────────────────────────────────────────────── */
export function Callout({ intent = 'info', title, children }: {
  intent?: Intent & ('info' | 'success' | 'warning' | 'danger');
  title: string; children: React.ReactNode;
}) {
  return (
    <div className={cn('callout', intent !== 'info' && `callout-${intent}`)}>
      <b className="callout-title">{title}</b>
      <span style={{ color: 'var(--muted)' }}>{children}</span>
    </div>
  );
}

/* ── Progress ───────────────────────────────────────────────────────────── */
export function Progress({ value, blocked, className }: { value: number; blocked?: boolean; className?: string }) {
  return (
    <div className={cn('progress', blocked && 'blocked', className)}>
      <i style={{ width: `${Math.min(100, Math.max(0, value))}%` }} />
    </div>
  );
}

/* ── Skeleton ───────────────────────────────────────────────────────────── */
export function Skeleton({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return <div className={cn('skeleton', className)} style={style} />;
}

/* ── RidTag ─────────────────────────────────────────────────────────────── */
export function RidTag({ rid, className }: { rid: string; className?: string }) {
  return (
    <span className={cn('rid', className)} title={rid} style={{ fontFamily: 'var(--mono)', cursor: 'default' }}>
      {rid}
    </span>
  );
}

/* ── Kicker / SectionLabel ──────────────────────────────────────────────── */
export function Kicker({ children, className }: { children: React.ReactNode; className?: string }) {
  return <div className={cn('kicker', className)}>{children}</div>;
}
