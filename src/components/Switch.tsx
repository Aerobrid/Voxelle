// Toggle switch: a real checkbox (keyboard + screen-reader behavior intact), styled as a switch.
export function Switch({
  checked,
  onChange,
  disabled,
  label,
  title,
}: {
  checked: boolean;
  onChange?: (v: boolean) => void;
  disabled?: boolean;
  label: string;
  title?: string;
}) {
  return (
    <span className="switch" title={title}>
      <input
        type="checkbox"
        role="switch"
        aria-label={label}
        checked={checked}
        disabled={disabled}
        onChange={(e) => onChange?.(e.target.checked)}
      />
      <span className="switch__track" />
    </span>
  );
}
