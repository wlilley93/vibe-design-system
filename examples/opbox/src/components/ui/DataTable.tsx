'use client';
/**
 * Opbox Design System — DataTable
 * The workhorse: hairline rows, mono identifier columns, right-aligned numbers,
 * compact density, status badges, selection, skeleton loading, empty state.
 */
import * as React from 'react';
import { clsx } from 'clsx';
import { StatusBadge } from './ui';
import { EntityIcon } from './EntityIcon';
import type { ObjectTypeId } from './EntityIcon';
import { Skeleton } from './ui';

const cn = clsx;

export type Column<T> = {
  key: string;
  label: string;
  align?: 'left' | 'right';
  mono?: boolean;          // render value in mono
  muted?: boolean;         // render value in muted tone
  width?: string;          // css grid width, e.g. '2fr' | '120px'
  render?: (row: T) => React.ReactNode;
};

export type Row = { id: string; status?: string; type?: ObjectTypeId;[k: string]: unknown };

export function DataTable<T extends Row>({
  columns, rows, density = 'compact', selectedId, onRowClick, loading, empty,
}: {
  columns: Column<T>[];
  rows: T[];
  density?: 'compact' | 'default';
  selectedId?: string;
  onRowClick?: (row: T) => void;
  loading?: boolean;
  empty?: { title: string; body?: string; type?: ObjectTypeId };
}) {
  const gridTemplate = columns.map(c => c.width ?? '1fr').join(' ');

  if (loading) {
    return (
      <div className="panel">
        <div className="panel-header">LOADING</div>
        <div style={{ padding: 12, display: 'grid', gap: 8 }}>
          {Array.from({ length: 5 }).map((_, i) => <Skeleton key={i} style={{ height: 18, width: `${90 - i * 12}%` }} />)}
        </div>
      </div>
    );
  }

  if (rows.length === 0 && empty) {
    return (
      <div className="panel" style={{ padding: 32, textAlign: 'center' }}>
        {empty.type && <div style={{ opacity: .4, marginBottom: 8 }}><EntityIcon type={empty.type} size={24} /></div>}
        <div style={{ fontSize: 12.5, fontWeight: 500 }}>{empty.title}</div>
        {empty.body && <div style={{ fontSize: 11, color: 'var(--muted)', marginTop: 2 }}>{empty.body}</div>}
      </div>
    );
  }

  return (
    <div className="panel">
      <div style={{ display: 'grid', gridTemplateColumns: gridTemplate, padding: '0 14px', height: 32, alignItems: 'center', background: 'var(--surface)', borderBottom: '1px solid var(--border)' }}>
        {columns.map(c => (
          <span key={c.key} style={{ fontFamily: 'var(--mono)', fontSize: 9, letterSpacing: '.1em', textTransform: 'uppercase', color: 'var(--muted)', textAlign: c.align ?? 'left' }}>
            {c.label}
          </span>
        ))}
      </div>
      {rows.map(row => {
        const selected = row.id === selectedId;
        return (
          <div
            key={row.id}
            onClick={() => onRowClick?.(row)}
            className={cn('row-hover', onRowClick && 'cursor-pointer')}
            style={{
              display: 'grid', gridTemplateColumns: gridTemplate, padding: '0 14px',
              height: density === 'compact' ? 38 : 44, alignItems: 'center',
              borderBottom: '1px solid var(--border)',
              background: selected ? 'var(--accent-soft)' : undefined,
              borderLeft: selected ? '2px solid var(--accent)' : '2px solid transparent',
            }}
          >
            {columns.map((c, ci) => {
              const content = c.render
                ? c.render(row)
                : ci === 0 && row.type
                  ? (
                    <span style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 0 }}>
                      <EntityIcon type={row.type} size={14} />
                      <span style={{ fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{String(row[c.key] ?? '')}</span>
                    </span>
                  )
                  : String(row[c.key] ?? '');
              return (
                <span key={c.key} style={{
                  textAlign: c.align ?? 'left',
                  fontFamily: c.mono ? 'var(--mono)' : undefined,
                  fontSize: c.mono ? 10.5 : undefined,
                  color: c.muted ? 'var(--muted)' : undefined,
                  overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                  fontVariantNumeric: 'tabular-nums',
                }}>
                  {c.key === 'status' && row.status ? <StatusBadge status={row.status} /> : content}
                </span>
              );
            })}
          </div>
        );
      })}
    </div>
  );
}
