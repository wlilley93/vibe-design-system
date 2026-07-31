'use client';
/**
 * Opbox Design System — ObjectInspector (the signature overlay)
 * A right-side slide-over that resolves any object and lets the user traverse
 * linked objects via a breadcrumb stack. Mount the provider at the app shell;
 * call useInspector() from anywhere to open an object.
 *
 * You supply a `resolve` function that maps an ObjRef to a display record
 * (properties + links). Everything else is handled here.
 */
import * as React from 'react';
import { createContext, useCallback, useContext, useState } from 'react';
import { clsx } from 'clsx';
import { EntityIcon, EntityIconTile } from './EntityIcon';
import type { ObjectTypeId } from './EntityIcon';
import { StatusBadge, RidTag } from './ui';

const cn = clsx;

export type ObjRef = { type: ObjectTypeId; id: string };
export type InspectorLink = { label: string; ref: ObjRef; title: string };
export type InspectorRecord = {
  typeName: string;
  title: string;
  status?: string;
  rid: string;
  props: [string, string][];
  links?: InspectorLink[];
  extras?: React.ReactNode;   // type-specific panels (timeline, steps, cap table…)
};

const Ctx = createContext<(o: ObjRef) => void>(() => {});
export const useInspector = () => useContext(Ctx);

export function InspectorProvider({ resolve, children }: {
  resolve: (ref: ObjRef) => InspectorRecord;
  children: React.ReactNode;
}) {
  const [stack, setStack] = useState<ObjRef[]>([]);
  const open = useCallback((o: ObjRef) => setStack(s => [...s, o]), []);
  const close = () => setStack([]);
  const back = () => setStack(s => s.slice(0, -1));
  const cur = stack[stack.length - 1];
  const rec = cur ? resolve(cur) : null;

  return (
    <Ctx.Provider value={open}>
      {children}
      {cur && rec && (
        <>
          <div onClick={close} style={{ position: 'fixed', inset: 0, background: 'rgba(17,20,24,.1)', zIndex: 40 }} />
          <aside style={{
            position: 'fixed', top: 0, right: 0, height: '100%', width: 400,
            background: 'var(--bg)', borderLeft: '1px solid var(--border)',
            boxShadow: 'var(--el3)', zIndex: 50, display: 'flex', flexDirection: 'column',
          }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '0 16px', height: 48, borderBottom: '1px solid var(--border)', flexShrink: 0 }}>
              {stack.length > 1
                ? <button onClick={back} className="icon-btn" aria-label="Back">←</button>
                : <div style={{ width: 32 }} />}
              <span style={{ fontFamily: 'var(--mono)', fontSize: 10, letterSpacing: '.18em', textTransform: 'uppercase', color: 'var(--muted)' }}>
                {rec.typeName}
              </span>
              <button onClick={close} className="icon-btn" aria-label="Close" style={{ marginLeft: 'auto' }}>✕</button>
            </div>
            <div style={{ flex: 1, overflowY: 'auto' }}>
              <div style={{ padding: 16, borderBottom: '1px solid var(--border)' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                  <EntityIconTile type={cur.type} />
                  <div style={{ minWidth: 0, flex: 1 }}>
                    <div style={{ fontSize: 14, fontWeight: 600, lineHeight: 1.3 }}>{rec.title}</div>
                    {rec.status && <div style={{ marginTop: 4 }}><StatusBadge status={rec.status} /></div>}
                  </div>
                </div>
                <RidTag rid={rec.rid} className="rid" />
              </div>
              <div style={{ padding: 16, borderBottom: '1px solid var(--border)' }}>
                <div className="panel-header" style={{ margin: '0 0 10px', border: 'none', background: 'none', padding: 0, height: 'auto' }}>PROPERTIES</div>
                <div className="panel">
                  {rec.props.map(([k, v]) => (
                    <div key={k} style={{ display: 'grid', gridTemplateColumns: '130px 1fr', gap: 12, padding: '7px 12px', borderBottom: '1px solid var(--border)' }}>
                      <span style={{ fontFamily: 'var(--mono)', fontSize: 10.5, color: 'var(--muted)' }}>{k}</span>
                      <span style={{ fontFamily: 'var(--mono)', fontSize: 11.5, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{v}</span>
                    </div>
                  ))}
                </div>
              </div>
              {rec.links && rec.links.length > 0 && (
                <div style={{ padding: 16 }}>
                  <div className="panel-header" style={{ margin: '0 0 10px', border: 'none', background: 'none', padding: 0, height: 'auto' }}>LINKED OBJECTS · TRAVERSE</div>
                  {rec.links.map((l, i) => (
                    <button key={i} onClick={() => open(l.ref)}
                      style={{
                        width: '100%', display: 'flex', alignItems: 'center', gap: 10, textAlign: 'left',
                        border: '1px solid var(--border)', borderRadius: 8, padding: '9px 12px', marginBottom: 6,
                        background: 'var(--bg)', cursor: 'pointer', transition: 'border-color .12s',
                      }}
                      onMouseEnter={e => (e.currentTarget.style.borderColor = 'var(--accent)')}
                      onMouseLeave={e => (e.currentTarget.style.borderColor = 'var(--border)')}
                    >
                      <EntityIcon type={l.ref.type} size={14} />
                      <span style={{ fontFamily: 'var(--mono)', fontSize: 10, color: 'var(--muted)' }}>—{l.label}→</span>
                      <span style={{ fontSize: 11.5, fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{l.title}</span>
                      <span style={{ marginLeft: 'auto', color: 'var(--muted)', fontSize: 12 }}>↗</span>
                    </button>
                  ))}
                </div>
              )}
              {rec.extras}
            </div>
          </aside>
        </>
      )}
    </Ctx.Provider>
  );
}
