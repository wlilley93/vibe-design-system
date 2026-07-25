import type { ReactNode } from 'react'

export type CardProps = {
  children: ReactNode
  title: string
  footer?: ReactNode
}

export function Card({ children, title, footer }: CardProps) {
  return (
    <section className="sf-card">
      <h2 className="sf-card__title">{title}</h2>
      <div className="sf-card__body">{children}</div>
      {footer ? <div className="sf-card__footer">{footer}</div> : null}
    </section>
  )
}
