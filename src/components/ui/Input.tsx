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
        "h-8 w-full rounded-[4px] border bg-surface px-3 text-[14px] text-primary",
        "placeholder:text-secondary transition-colors duration-100",
        "hover:bg-surface-2 focus:bg-bg focus:outline-none focus-visible:outline-none",
        // Windows draws focus as a thicker accent line along the bottom edge.
        invalid
          ? "border-danger border-b-2"
          : "border-border border-b-border-strong focus:border-b-2 focus:border-b-accent",
        mono && "font-mono",
        className,
      )}
      {...rest}
    />
  );
});
