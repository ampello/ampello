// SPDX-License-Identifier: GPL-3.0-or-later
export const isMac =
  typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

// The modifier's name as this platform writes it in menus and hints.
export const modKey = isMac ? "⌘" : "Ctrl";
