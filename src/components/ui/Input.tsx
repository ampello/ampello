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
        "h-8 w-full rounded-[6px] border bg-surface px-2.5 text-[13px] text-primary",
        "placeholder:text-muted transition-colors duration-100",
        "focus:outline-none focus-visible:outline-none",
        invalid
          ? "border-danger focus:border-danger"
          : "border-border focus:border-accent",
        mono && "font-mono",
        className,
      )}
      {...rest}
    />
  );
});
