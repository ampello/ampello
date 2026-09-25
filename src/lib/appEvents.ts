// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect } from "react";
import { isTauri } from "@/lib/ipc";
import { useDataStore } from "@/stores/dataStore";
import { useSettingsStore } from "@/stores/settingsStore";
import { useUiStore } from "@/stores/uiStore";

const EXPANDED = "ampello://expanded";
const SETTINGS_CHANGED = "ampello://settings-changed";
const OPEN_SETTINGS = "ampello://open-settings";
export const LIBRARY_CHANGED = "ampello://library-changed";
const EXTERNAL_CHANGE = "ampello://external-change";

export function useAppEvents() {
  useEffect(() => {
    if (!isTauri()) return;

    let cancelled = false;
    const stoppers: Array<() => void> = [];
    let usageTimer: ReturnType<typeof setTimeout> | undefined;

    let pending = false;

    const isVisible = () => document.visibilityState === "visible";

    const refreshNow = () => {
      pending = false;
      void useDataStore.getState().refresh().catch(() => undefined);
    };

    const refreshUsage = () => {
      if (!isVisible()) {
        pending = true;
        return;
      }
      if (usageTimer) clearTimeout(usageTimer);
      usageTimer = setTimeout(refreshNow, 1200);
    };

    const onVisibilityChange = () => {
      if (isVisible() && pending) refreshNow();
    };
    document.addEventListener("visibilitychange", onVisibilityChange);

    const reloadSettings = () => {
      void useSettingsStore.getState().load();
    };

    const openSettings = () => {
      useUiStore.getState().setView("settings");
    };

    // The library was exchanged underneath us, so nothing currently on screen
    // belongs to it: reload snippets, collections and settings together.
    const reloadLibrary = () => {
      void useDataStore.getState().load().catch(() => undefined);
      void useSettingsStore.getState().load();
    };

    // Another copy of Ampello edited the shared library. A quiet refresh, not
    // a reload: it must not blank the list or disturb an open editor.
    const reloadExternal = () => {
      void useDataStore.getState().refresh().catch(() => undefined);
      void useSettingsStore.getState().load();
    };

    void import("@tauri-apps/api/event").then(async ({ listen }) => {
      const subscriptions = await Promise.all([
        listen(EXPANDED, refreshUsage),
        listen(SETTINGS_CHANGED, reloadSettings),
        listen(OPEN_SETTINGS, openSettings),
        listen(LIBRARY_CHANGED, reloadLibrary),
        listen(EXTERNAL_CHANGE, reloadExternal),
      ]);
      if (cancelled) subscriptions.forEach((stop) => stop());
      else stoppers.push(...subscriptions);
    });

    return () => {
      cancelled = true;
      if (usageTimer) clearTimeout(usageTimer);
      document.removeEventListener("visibilitychange", onVisibilityChange);
      stoppers.forEach((stop) => stop());
    };
  }, []);
}
