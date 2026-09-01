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
  "inline-flex items-center justify-center gap-1.5 rounded-[6px] font-medium " +
  "whitespace-nowrap select-none " +
  "transition-[background-color,color,border-color,transform] duration-100 " +
  "active:scale-[0.98] " +
  "disabled:pointer-events-none disabled:opacity-45";

const variants: Record<Variant, string> = {
  primary:
    "bg-accent text-accent-contrast hover:bg-accent-hover active:bg-accent-active",
  secondary:
    "bg-surface text-primary border border-border hover:bg-surface-2 " +
    "active:bg-surface-3",
  ghost: "text-secondary hover:bg-surface-2 hover:text-primary active:bg-surface-3",
  danger: "bg-danger text-white hover:bg-danger-hover",
};

const sizes: Record<Size, string> = {
  sm: "h-7 px-2.5 text-[13px]",
  md: "h-8 px-3 text-[13px]",
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
