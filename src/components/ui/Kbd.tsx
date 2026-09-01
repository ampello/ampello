// SPDX-License-Identifier: GPL-3.0-or-later
import { cn } from "@/lib/cn";

export function Kbd({ keys, className }: { keys: string; className?: string }) {
  return (
    <span className={cn("inline-flex items-center gap-0.5", className)}>
      {keys.split(" ").map((key) => (
        <kbd
          key={key}
          className={cn(
            "min-w-[18px] rounded-[4px] border border-border bg-surface-2 px-1",
            "text-center font-sans text-[10.5px] leading-[17px] text-muted",
          )}
        >
          {key}
        </kbd>
      ))}
    </span>
  );
}
