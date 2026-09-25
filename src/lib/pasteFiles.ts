// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useRef } from "react";

// Files on the clipboard - a screenshot, an image copied from a browser, or
// files copied in the file manager. The web view hands these over as `File`
// objects (contents, no path), which is why they are sent to the core as bytes.
//
// Only a paste that actually carries files is taken over. Text pastes fall
// through untouched, so Ctrl/Cmd+V in the editor still pastes text.
export function usePasteFiles(onFiles: (files: File[]) => void): void {
  const handler = useRef(onFiles);
  handler.current = onFiles;

  useEffect(() => {
    const onPaste = (event: ClipboardEvent) => {
      const data = event.clipboardData;
      if (!data) return;

      const files: File[] = [];
      for (const item of Array.from(data.items ?? [])) {
        if (item.kind !== "file") continue;
        const file = item.getAsFile();
        if (file) files.push(file);
      }
      if (files.length === 0) return;

      event.preventDefault();
      event.stopPropagation();
      handler.current(files);
    };

    // Capture phase: the code editor would otherwise swallow the event first.
    document.addEventListener("paste", onPaste, true);
    return () => document.removeEventListener("paste", onPaste, true);
  }, []);
}

const EXTENSIONS: Record<string, string> = {
  "image/png": "png",
  "image/jpeg": "jpg",
  "image/gif": "gif",
  "image/webp": "webp",
  "image/bmp": "bmp",
  "image/svg+xml": "svg",
};

// A pasted screenshot arrives named "image.png" every time; give each its own
// name so several pastes do not look like one file.
export function nameForPasted(file: File, index: number): string {
  const generic = !file.name || /^image\.\w+$/i.test(file.name);
  if (!generic) return file.name;

  const extension = EXTENSIONS[file.type] ?? "bin";
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  const stamp =
    `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}-` +
    `${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
  return `pasted-${stamp}${index > 0 ? `-${index + 1}` : ""}.${extension}`;
}
