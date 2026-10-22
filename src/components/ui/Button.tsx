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

// Fluent buttons: 4px corners, and a slightly darker bottom edge that gives
// them the faint raised look of Windows 11 controls.
const base =
  "inline-flex items-center justify-center gap-2 rounded-[4px] " +
  "whitespace-nowrap select-none " +
  "transition-[background-color,color,border-color] duration-100 " +
  "focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-primary " +
  "disabled:pointer-events-none disabled:opacity-40";

const variants: Record<Variant, string> = {
  primary:
    "bg-accent text-accent-contrast border border-black/10 border-b-black/25 " +
    "hover:bg-accent-hover active:bg-accent-active active:border-b-black/10",
  secondary:
    "bg-surface text-primary border border-border border-b-border-strong " +
    "hover:bg-surface-2 active:bg-surface-3 active:text-secondary active:border-b-border",
  ghost:
    "text-primary hover:bg-accent-soft active:bg-accent-soft-hover",
  danger:
    "bg-danger text-white border border-black/10 border-b-black/25 hover:bg-danger-hover",
};

const sizes: Record<Size, string> = {
  sm: "h-7 px-3 text-[13px]",
  md: "h-8 px-4 text-[14px]",
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
