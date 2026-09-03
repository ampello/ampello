// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useState } from "react";
import * as ipc from "@/lib/ipc";
import { LIBRARY_CHANGED } from "@/lib/appEvents";

export function StatusBar() {
  const [path, setPath] = useState<string | null>(null);

  // The library can be exchanged from Settings while this is on screen, so the
  // path is re-read rather than captured once.
  useEffect(() => {
    if (!ipc.isTauri()) return;
    let cancelled = false;
    let stop: (() => void) | undefined;

    const load = () => {
      ipc
        .libraryInfo()
        .then((info) => {
          if (!cancelled) setPath(info.path);
        })
        .catch(() => undefined);
    };

    load();
    void import("@tauri-apps/api/event")
      .then(async ({ listen }) => {
        const off = await listen(LIBRARY_CHANGED, load);
        if (cancelled) off();
        else stop = off;
      })
      .catch(() => undefined);

    return () => {
      cancelled = true;
      stop?.();
    };
  }, []);

  return (
    <footer className="flex h-7 shrink-0 items-center justify-between gap-6 border-t border-border px-4 text-[11.5px] text-secondary">
      <span className="shrink-0">Ctrl+N new · Ctrl+K search · Ctrl+S save</span>
      {path ? (
        <span className="truncate" title={path}>
          {path}
        </span>
      ) : null}
    </footer>
  );
}
