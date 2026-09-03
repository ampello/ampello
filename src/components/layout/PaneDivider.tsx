// SPDX-License-Identifier: GPL-3.0-or-later
import type { PointerEvent as ReactPointerEvent, KeyboardEvent as ReactKeyboardEvent } from "react";
import { cn } from "@/lib/cn";

// Dragging the divider past this distance from the left edge hands the whole
// area to the editor, which is what "all the way across" should mean.
const EXPAND_AT = 140;
const KEY_STEP = 24;

interface PaneDividerProps {
  width: number;
  expanded: boolean;
  minPane: number;
  minList: number;
  containerRef: React.RefObject<HTMLElement | null>;
  onResize: (width: number) => void;
  onExpandedChange: (expanded: boolean) => void;
}

export function PaneDivider({
  width,
  expanded,
  minPane,
  minList,
  containerRef,
  onResize,
  onExpandedChange,
}: PaneDividerProps) {
  const startDrag = (event: ReactPointerEvent<HTMLDivElement>) => {
    const container = containerRef.current;
    if (!container || event.button !== 0) return;
    event.preventDefault();

    const rect = container.getBoundingClientRect();
    const move = (moveEvent: PointerEvent) => {
      if (moveEvent.clientX - rect.left < EXPAND_AT) {
        onExpandedChange(true);
        return;
      }
      onExpandedChange(false);
      const next = rect.right - moveEvent.clientX;
      onResize(Math.min(Math.max(next, minPane), rect.width - minList));
    };

    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", stop);
      document.body.style.removeProperty("cursor");
      document.body.style.removeProperty("user-select");
    };

    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
  };

  const onKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    const container = containerRef.current;
    if (!container) return;
    const available = container.getBoundingClientRect().width;

    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      event.preventDefault();
      if (expanded) {
        if (event.key === "ArrowRight") onExpandedChange(false);
        return;
      }
      const next = width + (event.key === "ArrowLeft" ? KEY_STEP : -KEY_STEP);
      onResize(Math.min(Math.max(next, minPane), available - minList));
    } else if (event.key === "Home") {
      event.preventDefault();
      onExpandedChange(true);
    } else if (event.key === "End") {
      event.preventDefault();
      onExpandedChange(false);
    }
  };

  return (
    <div
      role="separator"
      aria-orientation="vertical"
      aria-label="Resize the editor pane"
      aria-valuenow={expanded ? 100 : Math.round(width)}
      tabIndex={0}
      onPointerDown={startDrag}
      onKeyDown={onKeyDown}
      onDoubleClick={() => onExpandedChange(!expanded)}
      className={cn(
        "group relative w-px shrink-0 cursor-col-resize bg-border",
        "focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent",
      )}
    >
      {/* The visible line stays hairline-thin; this widens only what the
          pointer has to hit. */}
      <div className="absolute inset-y-0 -left-1 -right-1 z-10" />
      <div
        className={cn(
          "pointer-events-none absolute inset-y-0 -left-px -right-px",
          "bg-accent opacity-0 transition-opacity duration-150 group-hover:opacity-100",
        )}
      />
    </div>
  );
}
