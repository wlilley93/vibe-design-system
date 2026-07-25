import type { ReactNode } from 'react'

export type ButtonProps = {
  children: ReactNode
  variant?: 'primary' | 'secondary' | 'ghost'
  size?: 'sm' | 'md'
  disabled?: boolean
  onClick?: () => void
}

export function Button({
  children,
  variant = 'primary',
  size = 'md',
  disabled = false,
  onClick,
}: ButtonProps) {
  return (
    <button
      type="button"
      className={`sf-button sf-button--${variant} sf-button--${size}`}
      disabled={disabled}
      onClick={onClick}
    >
      {children}
    </button>
  )
}
