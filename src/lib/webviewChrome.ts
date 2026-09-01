// SPDX-License-Identifier: GPL-3.0-or-later
function isEditable(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;

  return Boolean(target.closest(".cm-editor"));
}

// Removes the web view's own page affordances - context menu, reload, print,
// zoom - which mean nothing in a desktop application and lose work when a
// refresh discards an open editor. Developer shortcuts survive in dev builds.
export function installWebviewChrome(): void {
  const development = import.meta.env.DEV;

  document.addEventListener("contextmenu", (event) => {
    if (isEditable(event.target)) return;
    event.preventDefault();
  });

  document.addEventListener(
    "keydown",
    (event) => {
      if (event.defaultPrevented) return;

      const mod = event.ctrlKey || event.metaKey;
      const key = event.key.toLowerCase();

      if (key === "f5" || (mod && key === "r")) {
        event.preventDefault();
        return;
      }

      if (mod && key === "p" && !event.shiftKey) {
        event.preventDefault();
        return;
      }

      if (mod && (key === "+" || key === "=" || key === "-" || key === "0")) {
        event.preventDefault();
        return;
      }

      if (development) return;

      if (key === "f12" || (mod && event.shiftKey && (key === "i" || key === "j" || key === "c"))) {
        event.preventDefault();
      }
    },

    { capture: true },
  );

  for (const name of ["dragover", "drop"] as const) {
    document.addEventListener(name, (event) => event.preventDefault());
  }
}
