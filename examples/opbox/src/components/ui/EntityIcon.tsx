'use client';
/**
 * Opbox Design System — EntityIcon
 * One icon per object type, drawn from a single registry so an object is
 * visually identical across graph, lists, inspectors, and portal previews.
 * No runtime dependency. Sizes: 14–16 list · 18–19 graph · 20 inspector.
 */
import type { ReactNode } from 'react';

export type ObjectTypeId =
  | 'entity' | 'stakeholder' | 'matter' | 'share-class' | 'holding'
  | 'convertible' | 'vesting-term' | 'submission' | 'portal' | 'invoice'
  | 'ubo' | 'document' | 'agent' | 'workflow';

export const ENTITY_HUE: Record<ObjectTypeId, string> = {
  entity: '#1677ff', stakeholder: '#7c9bff', matter: '#e065b0',
  'share-class': '#f97316', holding: '#f59e0b', convertible: '#0e9f6e',
  'vesting-term': '#8b5cf6', submission: '#64748b', portal: '#1677ff',
  invoice: '#0e9f6e', ubo: '#0e9f6e', document: '#64748b',
  agent: '#1677ff', workflow: '#0e9f6e',
};

const ICONS: Record<ObjectTypeId, ReactNode> = {
  entity: (<><polygon points="14,3 24,9 24,19 14,25 4,19 4,9" /><polygon points="14,8.5 19,11.5 19,16.5 14,19.5 9,16.5 9,11.5" opacity=".5" /></>),
  stakeholder: (<><circle cx="14" cy="10" r="4" /><path d="M5 24c1.5-5 5-7 9-7s7.5 2 9 7" /></>),
  matter: (<><rect x="4" y="9" width="20" height="13" rx="2" /><path d="M10 9V7a4 4 0 018 0v2" /></>),
  'share-class': (<><circle cx="14" cy="14" r="10" /><path d="M14 4v10l7 7" opacity=".8" /></>),
  holding: (<><circle cx="14" cy="14" r="9" /><circle cx="14" cy="14" r="2.6" fill="currentColor" stroke="none" /></>),
  convertible: (<><path d="M4 14h9" /><path d="M9 9l5 5-5 5" /><circle cx="20" cy="14" r="5" /></>),
  'vesting-term': (<><circle cx="14" cy="14" r="10" /><path d="M14 14V6M14 14l6 3" opacity=".8" /></>),
  submission: (<><path d="M5 17h18M7 17v4h14v-4" /><path d="M14 4v9M10 9l4 4 4-4" /></>),
  portal: (<><rect x="5" y="4" width="18" height="20" rx="2" /><circle cx="18" cy="14" r="1.4" fill="currentColor" stroke="none" /></>),
  invoice: (<><rect x="6" y="4" width="16" height="20" rx="2" /><path d="M10 10h8M10 14h8M10 18h5" /></>),
  ubo: (<><circle cx="14" cy="14" r="9" /><circle cx="14" cy="14" r="4" opacity=".6" /><circle cx="14" cy="14" r="1.4" fill="currentColor" stroke="none" /></>),
  document: (<><path d="M8 3h8l5 5v17H8z" /><path d="M16 3v5h5" /><path d="M11 14h7M11 18h5" /></>),
  agent: (<><polygon points="14,4 22,9 22,19 14,24 6,19 6,9" /><circle cx="11" cy="13" r="1.2" fill="currentColor" stroke="none" /><circle cx="17" cy="13" r="1.2" fill="currentColor" stroke="none" /><path d="M11 17h6" /></>),
  workflow: (<><circle cx="6" cy="6" r="2.5" /><circle cx="6" cy="22" r="2.5" /><circle cx="22" cy="14" r="2.5" /><path d="M8.5 6.8 19.5 13M8.5 21.2 19.5 15" /></>),
};

export function EntityIcon({ type, size = 16 }: { type: ObjectTypeId; size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 28 28" fill="none"
      stroke={ENTITY_HUE[type]} strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"
      style={{ color: ENTITY_HUE[type] }}>
      {ICONS[type]}
    </svg>
  );
}

/** Bordered, tinted tile used in headers / inspectors. */
export function EntityIconTile({ type, size = 40 }: { type: ObjectTypeId; size?: number }) {
  const hue = ENTITY_HUE[type];
  return (
    <div style={{
      width: size, height: size, borderRadius: 10, display: 'grid', placeItems: 'center',
      border: '1px solid var(--border)', background: hue + '0d', flexShrink: 0,
    }}>
      <EntityIcon type={type} size={size / 2} />
    </div>
  );
}
