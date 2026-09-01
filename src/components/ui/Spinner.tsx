// SPDX-License-Identifier: GPL-3.0-or-later
import { cn } from "@/lib/cn";

export function Spinner({ className }: { className?: string }) {
  return (
    <span
      role="status"
      aria-label="Loading"
      className={cn(
        "inline-block h-3.5 w-3.5 animate-spin rounded-full",
        "border-[1.5px] border-border border-t-accent",
        className,
      )}
    />
  );
}
