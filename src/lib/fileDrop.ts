// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useRef, useState } from "react";
import { isTauri } from "@/lib/ipc";

// Uses Tauri's native drop rather than the web view's, because HTML5 drops give
// `File` objects without a path and the attachment store takes files by path.
// Requires `dragDropEnabled` on the window.
export function useFileDrop(onDrop: (paths: string[]) => void): boolean {
  const [over, setOver] = useState(false);

  const handler = useRef(onDrop);
  handler.current = onDrop;

  useEffect(() => {
    if (!isTauri()) return;

    let cancelled = false;
    let unlisten: (() => void) | null = null;

    void import("@tauri-apps/api/webview")
      .then(({ getCurrentWebview }) =>
        getCurrentWebview().onDragDropEvent((event) => {
          const payload = event.payload;
          if (payload.type === "enter" || payload.type === "over") {
            setOver(true);
            return;
          }
          setOver(false);
          if (payload.type === "drop" && payload.paths.length > 0) {
            handler.current(payload.paths);
          }
        }),
      )
      .then((stop) => {
        if (cancelled) stop();
        else unlisten = stop;
      })
      .catch(() => {
      });

    return () => {
      cancelled = true;
      unlisten?.();
      setOver(false);
    };
  }, []);

  return over;
}
