export type TextFieldProps = {
  label: string
  value: string
  onChange: (next: string) => void
  invalid?: boolean
  disabled?: boolean
  describedBy?: string
}

export function TextField({
  label,
  value,
  onChange,
  invalid = false,
  disabled = false,
  describedBy,
}: TextFieldProps) {
  return (
    <label className="sf-field">
      <span className="sf-field__label">{label}</span>
      <input
        className="sf-field__input"
        value={value}
        disabled={disabled}
        aria-invalid={invalid}
        aria-describedby={describedBy}
        onChange={(event) => onChange(event.target.value)}
      />
    </label>
  )
}
