// SPDX-License-Identifier: GPL-3.0-or-later
import { forwardRef } from "react";
import type { ButtonHTMLAttributes } from "react";
import { cn } from "@/lib/cn";

type Variant = "primary" | "secondary" | "ghost" | "danger";
type Size = "sm" | "md";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

const base =
  "inline-flex items-center justify-center gap-1.5 rounded-[8px] font-medium " +
  "whitespace-nowrap select-none " +
  "transition-[background-color,color,border-color,box-shadow,transform] " +
  "duration-150 ease-[var(--ease-out)] active:scale-[0.985] " +
  "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent " +
  "disabled:pointer-events-none disabled:opacity-40";

const variants: Record<Variant, string> = {
  primary:
    "bg-accent text-accent-contrast shadow-[var(--shadow-sm)] " +
    "hover:bg-accent-hover active:bg-accent-active",
  secondary:
    "bg-surface text-primary border border-border shadow-[var(--shadow-sm)] " +
    "hover:bg-surface-2 hover:border-border-strong active:bg-surface-3",
  ghost:
    "text-secondary hover:bg-surface-2 hover:text-primary active:bg-surface-3",
  danger: "bg-danger text-white shadow-[var(--shadow-sm)] hover:bg-danger-hover",
};

const sizes: Record<Size, string> = {
  sm: "h-7 px-2.5 text-[12.5px]",
  md: "h-8 px-3.5 text-[13px]",
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { variant = "secondary", size = "md", className, type = "button", ...rest },
  ref,
) {
  return (
    <button
      ref={ref}
      type={type}
      className={cn(base, variants[variant], sizes[size], className)}
      {...rest}
    />
  );
});
