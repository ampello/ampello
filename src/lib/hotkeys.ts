// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect } from "react";
import { focusSearch } from "@/components/snippets/SearchField";
import { useUiStore } from "@/stores/uiStore";

function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
}

export function useGlobalHotkeys() {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return;
      const mod = event.ctrlKey || event.metaKey;
      if (!mod || event.altKey) return;
      const key = event.key.toLowerCase();
      const { view, openEditor, setView } = useUiStore.getState();

      if (key === "n" && !event.shiftKey) {
        if (view === "editor") return;
        event.preventDefault();
        openEditor(null);
        return;
      }

      if (key === "k") {
        if (view === "editor") return;
        event.preventDefault();
        setView("snippets");

        requestAnimationFrame(() => focusSearch());
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
}

export { isTypingTarget };
