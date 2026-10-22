// SPDX-License-Identifier: GPL-3.0-or-later
import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

export interface SegmentOption<T extends string> {
  value: T;
  label: string;
  icon?: ReactNode;
}

interface SegmentedControlProps<T extends string> {
  label: string;
  value: T;
  options: SegmentOption<T>[];
  onChange: (value: T) => void;
}

export function SegmentedControl<T extends string>({
  label,
  value,
  options,
  onChange,
}: SegmentedControlProps<T>) {
  return (
    <div
      role="radiogroup"
      aria-label={label}
      className="inline-flex items-center gap-0.5 rounded-[5px] border border-border bg-surface-2 p-[3px]"
    >
      {options.map((option) => {
        const selected = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={selected}
            onClick={() => onChange(option.value)}
            className={cn(
              "relative inline-flex h-7 items-center gap-1.5 rounded-[4px] px-3",
              "text-[13px] transition-colors duration-100",
              selected
                ? "bg-raised text-primary shadow-[var(--shadow-sm)]"
                : "text-secondary hover:bg-accent-soft hover:text-primary",
            )}
          >
            {option.icon}
            {option.label}
            {selected ? (
              <span
                aria-hidden="true"
                className="absolute bottom-0 left-1/2 h-[3px] w-4 -translate-x-1/2 rounded-full bg-accent"
              />
            ) : null}
          </button>
        );
      })}
    </div>
  );
}
