// SPDX-License-Identifier: GPL-3.0-or-later
import { forwardRef } from "react";
import type { InputHTMLAttributes } from "react";
import { cn } from "@/lib/cn";

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  invalid?: boolean;
  mono?: boolean;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  { className, invalid = false, mono = false, ...rest },
  ref,
) {
  return (
    <input
      ref={ref}
      aria-invalid={invalid || undefined}
      className={cn(
        "h-9 w-full rounded-[8px] border bg-surface px-3 text-[13px] text-primary",
        "placeholder:text-muted transition-[border-color,box-shadow] duration-150",
        "ease-[var(--ease-out)] focus:outline-none focus-visible:outline-none",
        invalid
          ? "border-danger focus:border-danger focus:shadow-[0_0_0_3px_var(--danger-soft)]"
          : "border-border focus:border-accent focus:shadow-[0_0_0_3px_var(--accent-ring)]",
        mono && "font-mono",
        className,
      )}
      {...rest}
    />
  );
});
