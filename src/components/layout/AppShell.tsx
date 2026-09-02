// SPDX-License-Identifier: GPL-3.0-or-later
import type { ReactNode } from "react";
import { Sidebar } from "./Sidebar";

export function AppShell({ children }: { children: ReactNode }) {
  return (
    <div className="flex h-full w-full overflow-hidden bg-sidebar text-primary">
      <Sidebar />
      <main
        className={
          "relative flex min-w-0 flex-1 flex-col overflow-hidden " +
          "rounded-l-[14px] border-l border-border bg-bg shadow-[var(--shadow-sm)]"
        }
      >
        {children}
      </main>
    </div>
  );
}
