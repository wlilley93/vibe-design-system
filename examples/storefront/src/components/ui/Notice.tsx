import type { ReactNode } from 'react'

/**
 * Superseded by Alert. Kept in the tree because a deprecated component is a
 * component that still exists: VDS S-9(6) makes retirement three phases, and
 * the file is only a defect once the record reaches `retired` (S-9(8)).
 *
 * No screen imports it, which is what `retirement_drain` measures.
 */
export type NoticeProps = {
  children: ReactNode
  heading: string
}

export function Notice({ children, heading }: NoticeProps) {
  return (
    <div className="sf-alert" role="status">
      <strong className="sf-alert__title">{heading}</strong>
      <p className="sf-alert__body">{children}</p>
    </div>
  )
}
