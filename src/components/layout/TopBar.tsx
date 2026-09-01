// SPDX-License-Identifier: GPL-3.0-or-later
import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

interface TopBarProps {
  title: ReactNode;

  meta?: ReactNode;
  center?: ReactNode;
  className?: string;
}

export function TopBar({ title, meta, center, className }: TopBarProps) {
  return (
    <header
      data-tauri-drag-region
      className={cn(
        "flex h-12 shrink-0 items-center gap-3 border-b border-border bg-bg",
        "pl-4 pr-[158px]",
        className,
      )}
    >
      <div data-tauri-drag-region className="flex min-w-0 shrink-0 items-baseline gap-2">
        <h1 className="truncate text-[13.5px] font-semibold tracking-[-0.005em] text-primary">
          {title}
        </h1>
        {meta ? <span className="text-[12px] text-muted">{meta}</span> : null}
      </div>

      {center ? (
        <div className="flex min-w-0 flex-1 justify-center">{center}</div>
      ) : (
        <div data-tauri-drag-region className="flex-1" />
      )}
    </header>
  );
}
