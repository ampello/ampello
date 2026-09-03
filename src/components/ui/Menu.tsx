// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Check, Ellipsis } from "lucide-react";
import { cn } from "@/lib/cn";
import { IconButton } from "./IconButton";

export interface MenuItem {
  label: string;
  onSelect: () => void;
  danger?: boolean;
  separatorBefore?: boolean;
  disabled?: boolean;
  // Present on items that toggle something, so the menu can carry state that
  // would otherwise need a control of its own.
  checked?: boolean;
}

interface MenuProps {
  label: string;
  items: MenuItem[];
  className?: string;
}

export function Menu({ label, items, className }: MenuProps) {
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState({ top: 0, left: 0 });
  const triggerRef = useRef<HTMLButtonElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    if (!open || !triggerRef.current) return;
    const rect = triggerRef.current.getBoundingClientRect();
    const width = 176;
    const estimatedHeight = items.length * 30 + 8;
    const left = Math.min(
      Math.max(8, rect.right - width),
      window.innerWidth - width - 8,
    );
    const below = rect.bottom + 4;
    const top =
      below + estimatedHeight > window.innerHeight - 8
        ? Math.max(8, rect.top - estimatedHeight - 4)
        : below;
    setPosition({ top, left });
  }, [open, items.length]);

  useEffect(() => {
    if (!open) return;
    const close = () => setOpen(false);
    const onPointerDown = (event: PointerEvent) => {
      const target = event.target as Node;
      if (panelRef.current?.contains(target)) return;
      if (triggerRef.current?.contains(target)) return;
      close();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        close();
        triggerRef.current?.focus();
      }
    };
    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("resize", close);
    window.addEventListener("scroll", close, true);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("resize", close);
      window.removeEventListener("scroll", close, true);
    };
  }, [open]);

  return (
    <>
      <IconButton
        ref={triggerRef}
        label={label}
        size="sm"
        active={open}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={(event) => {
          event.stopPropagation();
          setOpen((value) => !value);
        }}
        className={className}
      >
        <Ellipsis size={15} strokeWidth={1.75} />
      </IconButton>

      {open
        ? createPortal(
            <div
              ref={panelRef}
              role="menu"
              aria-label={label}
              style={{ top: position.top, left: position.left }}
              className={cn(
                "motion-pop fixed z-50 w-44 rounded-[8px] border border-border bg-surface p-1",
                "shadow-[var(--shadow-md)] origin-top-right",
              )}
            >
              {items.map((item, index) => (
                <div key={item.label}>
                  {item.separatorBefore && index > 0 ? (
                    <div className="my-1 border-t border-border" />
                  ) : null}
                  <button
                    type="button"
                    role={item.checked === undefined ? "menuitem" : "menuitemcheckbox"}
                    aria-checked={item.checked}
                    disabled={item.disabled}
                    onClick={(event) => {
                      event.stopPropagation();
                      setOpen(false);
                      item.onSelect();
                    }}
                    className={cn(
                      "flex h-[27px] w-full items-center rounded-[5px] px-2 text-left",
                      "text-[12.5px] transition-colors duration-75",
                      "disabled:pointer-events-none disabled:opacity-45",
                      item.danger
                        ? "text-danger hover:bg-danger-soft"
                        : "text-primary hover:bg-surface-2",
                    )}
                  >
                    {item.checked === undefined ? null : (
                      <span className="mr-1.5 flex w-3.5 shrink-0 justify-center">
                        {item.checked ? (
                          <Check size={12} strokeWidth={2.5} />
                        ) : null}
                      </span>
                    )}
                    {item.label}
                  </button>
                </div>
              ))}
            </div>,
            document.body,
          )
        : null}
    </>
  );
}
