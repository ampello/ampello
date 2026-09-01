// SPDX-License-Identifier: GPL-3.0-or-later
import { create } from "zustand";
import * as ipc from "@/lib/ipc";
import type { Appearance, Settings } from "@/lib/types";

const APPEARANCE_CACHE_KEY = "ampello.appearance";

export const DEFAULT_SETTINGS: Settings = {
  appearance: "system",
  expansionEnabled: true,
  launchAtStartup: false,
  globalShortcut: "CommandOrControl+Shift+Space",
  boundaryMode: "word",
  preserveBoundaryChar: true,
  restoreClipboard: true,
  injectionMode: "auto",
  typingSpeed: "balanced",
  clipboardShortcutEnabled: true,
  clipboardShortcut: "CommandOrControl+Shift+V",
  clipboardMode: "type",
  closeToTray: true,
  attachmentSettleMs: 500,
};

interface SettingsState {
  settings: Settings;
  status: "idle" | "loading" | "ready" | "error";
  error: string | null;
  load: () => Promise<void>;
  patch: (patch: Partial<Settings>) => Promise<void>;
  setAppearance: (appearance: Appearance) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: DEFAULT_SETTINGS,
  status: "idle",
  error: null,

  async load() {
    set({ status: "loading", error: null });
    try {
      const settings = await ipc.getSettings();
      cacheAppearance(settings.appearance);
      set({ settings, status: "ready" });
    } catch (error) {
      set({
        status: "error",
        error: error instanceof Error ? error.message : String(error),
      });
    }
  },

  async patch(patch) {
    const previous = get().settings;
    const next = { ...previous, ...patch };
    set({ settings: next });
    if (patch.appearance) cacheAppearance(patch.appearance);
    try {
      const confirmed = await ipc.updateSettings(patch);
      set({ settings: confirmed });
    } catch (error) {
      set({
        settings: previous,
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  },

  setAppearance(appearance) {
    return get().patch({ appearance });
  },
}));

function cacheAppearance(appearance: Appearance) {
  try {
    localStorage.setItem(APPEARANCE_CACHE_KEY, appearance);
  } catch {
  }
}

export function cachedAppearance(): Appearance {
  try {
    const value = localStorage.getItem(APPEARANCE_CACHE_KEY);
    if (value === "light" || value === "dark" || value === "system") return value;
  } catch {
  }
  return "system";
}
