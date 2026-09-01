// SPDX-License-Identifier: GPL-3.0-or-later
import type { ReactNode } from "react";
import { Sidebar } from "./Sidebar";

export function AppShell({ children }: { children: ReactNode }) {
  return (
    <div className="flex h-full w-full overflow-hidden bg-bg text-primary">
      <Sidebar />
      <div className="flex min-w-0 flex-1 flex-col">{children}</div>
    </div>
  );
}
