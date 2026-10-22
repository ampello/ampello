// SPDX-License-Identifier: GPL-3.0-or-later
import type { ReactNode } from "react";
import { PanelLeft } from "lucide-react";
import appIcon from "../../../src-tauri/icons/64x64.png";
import { cn } from "@/lib/cn";
import { isMac } from "@/lib/platform";
import { useUiStore } from "@/stores/uiStore";
import { IconButton } from "@/components/ui/IconButton";
import { Sidebar } from "./Sidebar";

// The Windows 11 layout: a title bar across the whole window, navigation down
// the left on the Mica-grey background, and the page on a lighter layer whose
// top-left corner is rounded.
export function AppShell({ children }: { children: ReactNode }) {
  const collapsed = useUiStore((s) => s.sidebarCollapsed);
  const toggle = useUiStore((s) => s.toggleSidebar);

  return (
    <div className="flex h-full w-full flex-col overflow-hidden bg-sidebar text-primary">
      <header
        data-tauri-drag-region
        className={cn(
          "flex h-12 shrink-0 items-center gap-1",
          isMac ? "pl-[84px]" : "pl-2",
        )}
      >
        <IconButton
          label={collapsed ? "Expand navigation" : "Collapse navigation"}
          onClick={toggle}
          className="h-9 w-10"
        >
          <PanelLeft size={16} strokeWidth={1.75} />
        </IconButton>
        <img
          src={appIcon}
          alt=""
          draggable={false}
          data-tauri-drag-region
          className="ml-1 h-4 w-4"
        />
        <span data-tauri-drag-region className="ml-2 text-[12px] text-primary">
          Ampello
        </span>
      </header>

      <div className="flex min-h-0 flex-1">
        <Sidebar />
        <main
          className={
            "relative flex min-w-0 flex-1 flex-col overflow-hidden " +
            "rounded-tl-[8px] border-l border-t border-card-border bg-bg"
          }
        >
          {children}
        </main>
      </div>
    </div>
  );
}
