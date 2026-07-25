import type { ReactNode } from 'react'

export type AlertProps = {
  children: ReactNode
  title: string
  tone?: 'info' | 'critical'
}

export function Alert({ children, title, tone = 'info' }: AlertProps) {
  return (
    <div className={`sf-alert sf-alert--${tone}`} role="status">
      <strong className="sf-alert__title">{title}</strong>
      <p className="sf-alert__body">{children}</p>
    </div>
  )
}
