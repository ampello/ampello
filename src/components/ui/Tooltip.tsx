// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { createPortal } from "react-dom";
import { cn } from "@/lib/cn";

const WIDTH = 320;
const DELAY_MS = 320;

export function Tooltip({
  content,
  children,
  className,
}: {
  content: string;
  children: ReactNode;
  className?: string;
}) {
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState({ top: 0, left: 0 });
  const anchor = useRef<HTMLSpanElement>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const show = () => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => {
      const element = anchor.current;
      if (!element) return;
      const rect = element.getBoundingClientRect();

      const left = Math.min(
        Math.max(8, rect.left),
        Math.max(8, window.innerWidth - WIDTH - 8),
      );

      const height = Math.ceil(content.length / 46) * 18 + 18;
      const below = rect.bottom + 6;
      const top =
        below + height > window.innerHeight - 8
          ? Math.max(8, rect.top - height - 6)
          : below;

      setPosition({ top, left });
      setOpen(true);
    }, DELAY_MS);
  };

  const hide = () => {
    if (timer.current) clearTimeout(timer.current);
    setOpen(false);
  };

  useEffect(() => {
    return () => {
      if (timer.current) clearTimeout(timer.current);
    };
  }, []);

  useEffect(() => {
    if (!open) return;
    const close = () => setOpen(false);
    window.addEventListener("scroll", close, true);
    window.addEventListener("resize", close);
    return () => {
      window.removeEventListener("scroll", close, true);
      window.removeEventListener("resize", close);
    };
  }, [open]);

  return (
    <>
      <span
        ref={anchor}
        tabIndex={0}
        onMouseEnter={show}
        onMouseLeave={hide}
        onFocus={show}
        onBlur={hide}
        className={cn(

          "cursor-help underline decoration-muted/60 decoration-dotted underline-offset-4",
          "transition-colors duration-100 hover:decoration-muted",
          className,
        )}
      >
        {children}
      </span>

      {open
        ? createPortal(
            <div
              role="tooltip"
              style={{ top: position.top, left: position.left, width: WIDTH }}
              className={cn(
                "motion-fade pointer-events-none fixed z-[60] rounded-[8px] border border-border",
                "bg-surface px-3 py-2.5 text-[12px] leading-relaxed text-secondary",
                "shadow-[var(--shadow-md)]",
              )}
            >
              {content}
            </div>,
            document.body,
          )
        : null}
    </>
  );
}
