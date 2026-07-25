import type { ReactNode } from 'react'

export type BadgeProps = {
  children: ReactNode
  tone?: 'neutral' | 'positive' | 'critical'
}

export function Badge({ children, tone = 'neutral' }: BadgeProps) {
  return <span className={`sf-badge sf-badge--${tone}`}>{children}</span>
}
