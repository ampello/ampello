// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { cn } from "@/lib/cn";
import { isTauri } from "@/lib/ipc";

export function WindowControls() {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    if (!isTauri()) return;
    let stop: (() => void) | undefined;
    let cancelled = false;

    void import("@tauri-apps/api/window").then(async ({ getCurrentWindow }) => {
      if (cancelled) return;
      const appWindow = getCurrentWindow();
      const read = () => void appWindow.isMaximized().then(setMaximized).catch(() => undefined);
      read();
      const unlisten = await appWindow.onResized(read);
      if (cancelled) unlisten();
      else stop = unlisten;
    });

    return () => {
      cancelled = true;
      stop?.();
    };
  }, []);

  const act = (name: "minimize" | "toggleMaximize" | "close") => {
    if (!isTauri()) return;
    void import("@tauri-apps/api/window").then(({ getCurrentWindow }) => {
      const appWindow = getCurrentWindow();

      const run =
        name === "minimize"
          ? appWindow.minimize()
          : name === "toggleMaximize"
            ? appWindow.toggleMaximize()
            : appWindow.close();
      void run.catch(() => undefined);
    });
  };

  return (
    <div className="fixed right-0 top-0 z-50 flex h-12 shrink-0">
      <CaptionButton label="Minimize" onClick={() => act("minimize")}>
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <path d="M0 5h10" stroke="currentColor" strokeWidth="1" />
        </svg>
      </CaptionButton>

      <CaptionButton
        label={maximized ? "Restore" : "Maximize"}
        onClick={() => act("toggleMaximize")}
      >
        {maximized ? (
          <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
            <path
              d="M2.5 2.5V0.5h7v7h-2"
              fill="none"
              stroke="currentColor"
              strokeWidth="1"
            />
            <rect
              x="0.5"
              y="2.5"
              width="7"
              height="7"
              fill="none"
              stroke="currentColor"
              strokeWidth="1"
            />
          </svg>
        ) : (
          <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
            <rect
              x="0.5"
              y="0.5"
              width="9"
              height="9"
              fill="none"
              stroke="currentColor"
              strokeWidth="1"
            />
          </svg>
        )}
      </CaptionButton>

      <CaptionButton label="Close" danger onClick={() => act("close")}>
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <path d="M0 0l10 10M10 0L0 10" stroke="currentColor" strokeWidth="1" />
        </svg>
      </CaptionButton>
    </div>
  );
}

function CaptionButton({
  label,
  danger = false,
  onClick,
  children,
}: {
  label: string;
  danger?: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className={cn(
        "inline-flex h-12 w-[46px] items-center justify-center",
        "text-secondary transition-colors duration-75",
        danger
          ? "hover:bg-titlebar-close hover:text-titlebar-close-text"
          : "hover:bg-surface-3 hover:text-primary",
      )}
    >
      {children}
    </button>
  );
}

export const CONTROLS_WIDTH = 3 * 46;
