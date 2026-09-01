// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect } from "react";
import { isTauri } from "@/lib/ipc";
import { useDataStore } from "@/stores/dataStore";
import { useSettingsStore } from "@/stores/settingsStore";
import { useUiStore } from "@/stores/uiStore";

const EXPANDED = "ampello://expanded";
const SETTINGS_CHANGED = "ampello://settings-changed";
const OPEN_SETTINGS = "ampello://open-settings";

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

    void import("@tauri-apps/api/event").then(async ({ listen }) => {
      const subscriptions = await Promise.all([
        listen(EXPANDED, refreshUsage),
        listen(SETTINGS_CHANGED, reloadSettings),
        listen(OPEN_SETTINGS, openSettings),
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
