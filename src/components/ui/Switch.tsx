// SPDX-License-Identifier: GPL-3.0-or-later
import { cn } from "@/lib/cn";

interface SwitchProps {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

export function Switch({ label, checked, onChange, disabled = false }: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        "relative inline-flex h-[22px] w-[38px] shrink-0 items-center rounded-full",
        "transition-colors duration-150",
        "disabled:pointer-events-none disabled:opacity-45",
        checked ? "bg-accent" : "bg-surface-3 border border-border",
      )}
    >
      <span
        className={cn(
          "block h-[16px] w-[16px] rounded-full bg-surface shadow-[var(--shadow-sm)]",
          "transition-transform duration-150",
          checked ? "translate-x-[19px]" : "translate-x-[3px]",
        )}
      />
    </button>
  );
}
