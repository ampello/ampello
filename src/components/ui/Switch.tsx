// SPDX-License-Identifier: GPL-3.0-or-later
import { cn } from "@/lib/cn";

interface SwitchProps {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  // Shows "On" or "Off" beside the switch, as Windows does in settings.
  showState?: boolean;
}

// The Windows 11 toggle: an outlined track with a small knob when off, a
// filled accent track when on, and a knob that grows under the pointer.
export function Switch({
  label,
  checked,
  onChange,
  disabled = false,
  showState = true,
}: SwitchProps) {
  return (
    <div className="inline-flex items-center gap-3">
      {showState ? (
        <span aria-hidden="true" className="w-6 text-right text-[14px] text-primary">
          {checked ? "On" : "Off"}
        </span>
      ) : null}
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-label={label}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={cn(
          "group relative inline-flex h-5 w-10 shrink-0 items-center rounded-full border",
          "transition-colors duration-150",
          "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary",
          "disabled:pointer-events-none disabled:opacity-40",
          checked
            ? "border-accent bg-accent hover:bg-accent-hover hover:border-accent-hover"
            : "border-secondary bg-transparent hover:bg-accent-soft",
        )}
      >
        <span
          className={cn(
            "block rounded-full transition-all duration-150 ease-[var(--ease-out)]",
            "h-3 w-3 group-hover:h-3.5 group-hover:w-3.5 group-active:w-4",
            checked ? "bg-accent-contrast" : "bg-knob-off",
            checked ? "translate-x-[23px] group-active:translate-x-[19px]" : "translate-x-[3px]",
          )}
        />
      </button>
    </div>
  );
}
