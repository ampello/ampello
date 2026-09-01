// SPDX-License-Identifier: GPL-3.0-or-later
import { Suspense, lazy, useEffect } from "react";
import * as ipc from "@/lib/ipc";
import { useToastStore } from "@/stores/toastStore";
import { AppShell } from "@/components/layout/AppShell";
import { WindowControls } from "@/components/layout/WindowControls";
import { Spinner } from "@/components/ui/Spinner";
import { Toaster } from "@/components/ui/Toaster";
import { DashboardView } from "@/views/DashboardView";
import { SnippetsView } from "@/views/SnippetsView";
import { SettingsView } from "@/views/SettingsView";
import { BootErrorView } from "@/views/BootErrorView";

const EditorView = lazy(() =>
  import("@/views/EditorView").then((module) => ({ default: module.EditorView })),
);
import { useSettingsStore } from "@/stores/settingsStore";
import { useDataStore } from "@/stores/dataStore";
import { useUiStore } from "@/stores/uiStore";
import { useThemeSync } from "@/theme/useTheme";
import { useGlobalHotkeys } from "@/lib/hotkeys";
import { useAppEvents } from "@/lib/appEvents";

export default function App() {
  const settingsStatus = useSettingsStore((s) => s.status);
  const settingsError = useSettingsStore((s) => s.error);
  const loadSettings = useSettingsStore((s) => s.load);
  const dataStatus = useDataStore((s) => s.status);
  const dataError = useDataStore((s) => s.error);
  const loadData = useDataStore((s) => s.load);
  const view = useUiStore((s) => s.view);
  const editingId = useUiStore((s) => s.editingId);

  useThemeSync();
  useGlobalHotkeys();
  useAppEvents();

  useEffect(() => {
    void loadSettings();
    void loadData();
  }, [loadSettings, loadData]);

  useEffect(() => {
    if (!ipc.isTauri()) return;
    ipc
      .databaseInfo()
      .then((info) => {
        if (info.recoveredFrom) {
          useToastStore
            .getState()
            .push(
              "error",
              "Ampello could not read your previous library and started a fresh one. Nothing was deleted. See Settings → Storage.",
            );
        }
      })
      .catch(() => undefined);
  }, []);

  const boot = async () => {
    await Promise.all([loadSettings(), loadData()]);
  };

  if (settingsStatus === "error" || dataStatus === "error") {
    return (
      <>
        <BootErrorView
          message={settingsError ?? dataError ?? "Unknown error."}
          onRetry={() => void boot()}
        />
        <WindowControls />
      </>
    );
  }

  if (settingsStatus !== "ready" || dataStatus !== "ready") {
    return (
      <div className="relative flex h-full w-full items-center justify-center bg-bg">
        <div
          data-tauri-drag-region
          className="absolute inset-x-0 top-0 h-12"
          aria-hidden="true"
        />
        <Spinner />
        <WindowControls />
      </div>
    );
  }

  return (
    <>
      <AppShell>
        <div key={view} className="motion-rise flex min-h-0 flex-1 flex-col">
        {view === "dashboard" ? <DashboardView /> : null}
        {view === "snippets" ? <SnippetsView /> : null}
        {view === "settings" ? <SettingsView /> : null}
        {view === "editor" && editingId ? (
          <Suspense
            fallback={
              <div className="flex flex-1 items-center justify-center">
                <Spinner />
              </div>
            }
          >
            <EditorView key={editingId} id={editingId} />
          </Suspense>
        ) : null}
        </div>
      </AppShell>
      <WindowControls />
      <Toaster />
    </>
  );
}
