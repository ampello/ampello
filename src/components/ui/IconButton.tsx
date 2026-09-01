// SPDX-License-Identifier: GPL-3.0-or-later
import { forwardRef } from "react";
import type { ButtonHTMLAttributes } from "react";
import { cn } from "@/lib/cn";

export interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  active?: boolean;
  size?: "sm" | "md";
}

export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(
  function IconButton(
    { label, active = false, size = "md", className, type = "button", ...rest },
    ref,
  ) {
    return (
      <button
        ref={ref}
        type={type}
        aria-label={label}
        title={label}
        aria-pressed={rest["aria-pressed"] ?? undefined}
        className={cn(
          "inline-flex items-center justify-center rounded-[6px]",
          "transition-[background-color,color,transform] duration-100 active:scale-[0.94]",
          "text-secondary hover:bg-surface-2 hover:text-primary active:bg-surface-3",
          "disabled:pointer-events-none disabled:opacity-45",
          active && "bg-surface-2 text-primary",
          size === "sm" ? "h-7 w-7" : "h-8 w-8",
          className,
        )}
        {...rest}
      />
    );
  },
);
