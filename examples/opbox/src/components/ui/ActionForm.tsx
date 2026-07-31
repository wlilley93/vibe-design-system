'use client';
/**
 * Opbox Design System — ActionForm
 * The form that is a pure function of an action type's parameter declarations.
 * Renders labeled controls per parameter, evaluates conditional visibility live,
 * and gates the submit button on submission criteria. Used in matters, portals,
 * and the Review Queue — never hand-build a form for a typed action.
 */
import * as React from 'react';
import { useMemo, useState } from 'react';
import { Button, Field, Input } from './ui';

export type ParamType = 'string' | 'decimal' | 'integer' | 'boolean' | 'date' | 'enum' | 'ref' | 'array';
export type Param = {
  name: string;
  type: ParamType;
  required?: boolean;
  label?: string;
  hint?: string;                 // e.g. "when ground=ownership"
  options?: string[];            // for enum
  default?: unknown;
  visibleWhen?: string;          // expression, e.g. "ground == 'ownership'"
  validate?: (v: unknown) => string | undefined; // error message or undefined
};
export type Criterion = { expr: string; message: string; evaluate: (values: Record<string, unknown>) => boolean };
export type ActionTypeDef = {
  rid: string;
  name: string;
  parameters: Param[];
  criteria: Criterion[];
};

/** Minimal, safe visibleWhen evaluator for "field == 'literal'" / "field != 'literal'" / "field" */
function evalVisible(expr: string | undefined, values: Record<string, unknown>): boolean {
  if (!expr) return true;
  const eq = expr.match(/^(\w+)\s*==\s*'([^']*)'$/);
  if (eq) return String(values[eq[1]]) === eq[2];
  const ne = expr.match(/^(\w+)\s*!=\s*'([^']*)'$/);
  if (ne) return String(values[ne[1]]) !== ne[2];
  return Boolean(values[expr.trim()]);
}

export function ActionForm({ actionType, onSubmit, submitLabel = 'Submit' }: {
  actionType: ActionTypeDef;
  onSubmit?: (values: Record<string, unknown>) => void;
  submitLabel?: string;
}) {
  const [values, setValues] = useState<Record<string, unknown>>(() =>
    Object.fromEntries(actionType.parameters.filter(p => p.default !== undefined).map(p => [p.name, p.default])));

  const visibleParams = useMemo(() => actionType.parameters.filter(p => evalVisible(p.visibleWhen, values)), [actionType, values]);
  const failing = actionType.criteria.find(c => !c.evaluate(values));
  const errors: Record<string, string | undefined> = {};
  visibleParams.forEach(p => { if (p.validate) errors[p.name] = p.validate(values[p.name]); });
  const hasError = Object.values(errors).some(Boolean);

  const set = (name: string, v: unknown) => setValues(s => ({ ...s, [name]: v }));

  return (
    <form
      onSubmit={e => { e.preventDefault(); if (!failing && !hasError) onSubmit?.(values); }}
      style={{ display: 'grid', gap: 12 }}
    >
      <div style={{ fontFamily: 'var(--mono)', fontSize: 9.5, letterSpacing: '.1em', textTransform: 'uppercase', color: 'var(--muted)' }}>
        {actionType.name} · {actionType.rid}
      </div>
      {visibleParams.map(p => (
        <Field key={p.name} label={p.label ?? p.name} required={p.required} hint={p.hint}>
          {p.type === 'enum' ? (
            <select className="input mono" value={String(values[p.name] ?? '')} onChange={e => set(p.name, e.target.value)}>
              <option value="">—</option>
              {p.options?.map(o => <option key={o} value={o}>{o}</option>)}
            </select>
          ) : p.type === 'boolean' ? (
            <input type="checkbox" checked={Boolean(values[p.name])} onChange={e => set(p.name, e.target.checked)} style={{ accentColor: 'var(--accent)' }} />
          ) : (
            <Input
              mono={p.type !== 'string'}
              type={p.type === 'date' ? 'date' : 'text'}
              value={String(values[p.name] ?? '')}
              onChange={e => set(p.name, p.type === 'decimal' || p.type === 'integer' ? e.target.value : e.target.value)}
              error={errors[p.name]}
              placeholder={p.type}
            />
          )}
        </Field>
      ))}
      <div>
        <Button
          variant="primary"
          type="submit"
          disabled={!!failing || hasError}
          blockedReason={failing?.message}
        >
          {submitLabel}
        </Button>
      </div>
    </form>
  );
}
