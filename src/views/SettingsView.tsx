// SPDX-License-Identifier: GPL-3.0-or-later
import { useCallback, useEffect, useState } from "react";
import type { ReactNode } from "react";
import { Monitor, Moon, Sun, TriangleAlert } from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { SegmentedControl } from "@/components/ui/SegmentedControl";
import { SettingsBlock, SettingsRow, SettingsSection } from "@/components/ui/SettingsSection";
import { Switch } from "@/components/ui/Switch";
import { Button } from "@/components/ui/Button";
import { Kbd } from "@/components/ui/Kbd";
import { ShortcutRecorder } from "@/components/ui/ShortcutRecorder";
import { useSettingsStore } from "@/stores/settingsStore";
import { useDataStore } from "@/stores/dataStore";
import { reportError, useToastStore } from "@/stores/toastStore";
import { cn } from "@/lib/cn";
import * as ipc from "@/lib/ipc";
import { MAX_SETTLE_MS, MIN_SETTLE_MS } from "@/lib/types";
import type {
  Appearance,
  BoundaryMode,
  DatabaseInfo,
  LibraryInfo,
  Diagnostics,
  EngineStatus,
  ClipboardMode,
  ImportReport,
  InjectionMode,
  TypingSpeed,
} from "@/lib/types";

const APPEARANCE_OPTIONS = [
  { value: "light" as const, label: "Light", icon: <Sun size={14} strokeWidth={1.75} /> },
  { value: "dark" as const, label: "Dark", icon: <Moon size={14} strokeWidth={1.75} /> },
  { value: "system" as const, label: "System", icon: <Monitor size={14} strokeWidth={1.75} /> },
];

const BOUNDARY_OPTIONS = [
  { value: "word" as const, label: "Word start" },
  { value: "anywhere" as const, label: "Anywhere" },
];

const INJECTION_OPTIONS = [
  { value: "auto" as const, label: "Auto" },
  { value: "paste" as const, label: "Paste" },
  { value: "type" as const, label: "Type" },
];

const TYPING_OPTIONS = [
  { value: "fast" as const, label: "Fast" },
  { value: "balanced" as const, label: "Balanced" },
  { value: "careful" as const, label: "Careful" },
];

const CLIPBOARD_OPTIONS = [
  { value: "type" as const, label: "Type" },
  { value: "paste" as const, label: "Paste" },
];

const SHORTCUTS: [string, string][] = [
  ["Ctrl N", "New snippet"],
  ["Ctrl F", "Find in the editor"],
  ["Ctrl K", "Search snippets"],
  ["Ctrl H", "Find and replace"],
  ["Ctrl S", "Save"],
  ["↑ ↓", "Move through the list"],
  ["Enter", "Open the selected snippet"],
  ["Del", "Delete the selected snippet"],
  ["Esc", "Close the editor or a dialog"],
  ["Esc", "Stop an expansion part-way through"],
];

export function SettingsView() {
  const settings = useSettingsStore((s) => s.settings);
  const patch = useSettingsStore((s) => s.patch);
  const setAppearance = useSettingsStore((s) => s.setAppearance);
  const error = useSettingsStore((s) => s.error);
  const pushToast = useToastStore((s) => s.push);

  const [engine, setEngine] = useState<EngineStatus | null>(null);
  const [info, setInfo] = useState<DatabaseInfo | null>(null);
  const [library, setLibrary] = useState<LibraryInfo | null>(null);
  const [pendingRestart, setPendingRestart] = useState(false);
  const [diag, setDiag] = useState<Diagnostics | null>(null);
  const [shortcutProblem, setShortcutProblem] = useState<string | null>(null);
  const [busy, setBusy] = useState<"export" | "import" | "library" | null>(null);
  const [report, setReport] = useState<ImportReport | null>(null);
  const [restarting, setRestarting] = useState(false);

  const readEngine = useCallback(() => {
    ipc.engineStatus().then(setEngine).catch(() => setEngine(null));
    ipc.shortcutError().then(setShortcutProblem).catch(() => setShortcutProblem(null));
  }, []);

  useEffect(() => {
    readEngine();
    ipc.databaseInfo().then(setInfo).catch(() => setInfo(null));
    ipc.libraryInfo().then(setLibrary).catch(() => setLibrary(null));
    ipc.diagnostics().then(setDiag).catch(() => setDiag(null));
  }, [readEngine]);

  const apply = (change: Parameters<typeof patch>[0]) => {
    void patch(change).then(readEngine).catch(reportError);
  };

  const restartEngine = () => {
    setRestarting(true);
    ipc
      .restartEngine()
      .then((status) => {
        setEngine(status);
        pushToast(
          "info",
          status.running ? "Expansion engine restarted." : "The engine could not be restarted.",
        );
      })
      .catch(reportError)
      .finally(() => setRestarting(false));
  };

  const runExport = (format: "yaml" | "json") => {
    setBusy("export");
    setReport(null);
    ipc
      .exportBackup(format)
      .then((result) => {
        if (result) {
          pushToast(
            "info",
            `Exported ${result.snippets} snippet${result.snippets === 1 ? "" : "s"} to ${result.path}`,
          );
        }
      })
      .catch(reportError)
      .finally(() => setBusy(null));
  };

  const pickLibrary = async () => {
    setBusy("library");
    try {
      const path = await ipc.chooseSharedLibrary();
      if (path) {
        setPendingRestart(true);
        pushToast("info", `Shared library set to ${path}`);
      }
    } catch (error) {
      reportError(error);
    } finally {
      setBusy(null);
    }
  };

  const revertLibrary = async () => {
    setBusy("library");
    try {
      await ipc.usePersonalLibrary();
      setPendingRestart(true);
      pushToast("info", "Ampello will use this account's own library.");
    } catch (error) {
      reportError(error);
    } finally {
      setBusy(null);
    }
  };

  const runImport = () => {
    setBusy("import");
    setReport(null);
    ipc

      .importBackup("skip")
      .then((result) => {
        if (!result) return;
        setReport(result);
        void useDataStore.getState().refresh();
        readEngine();
      })
      .catch(reportError)
      .finally(() => setBusy(null));
  };

  return (
    <>
      <TopBar title="Settings" />
      <main className="flex-1 overflow-y-auto">
        <div className="mx-auto w-full max-w-[840px] px-8 py-9">
          {error ? (
            <div className="mb-7 overflow-hidden rounded-[12px] border border-border">
              <Notice tone="danger">{error}</Notice>
            </div>
          ) : null}
          <SettingsSection title="Expansion">
            <SettingsBlock>
              <EngineBanner
                status={engine}
                restarting={restarting}
                onRestart={restartEngine}
              />
            </SettingsBlock>

            <SettingsRow
              label="Expand snippets"
              hint="Turns Ampello's system-wide expansion on and off. When it is off Ampello keeps running and your snippets are untouched, but nothing is watched for and nothing is replaced. The same switch appears in the notification-area menu."
              control={
                <Switch
                  label="Expand snippets"
                  checked={settings.expansionEnabled}
                  onChange={(enabled) => {
                    void ipc
                      .setExpansionEnabled(enabled)
                      .then(() => useSettingsStore.getState().load())
                      .then(readEngine)
                      .catch(reportError);
                  }}
                />
              }
            />

            <SettingsRow
              label="Trigger position"
              hint="How much of a word a trigger is allowed to interrupt. Word start requires the trigger to begin a new word, so typing “something:sig” is left alone and only “:sig” after a space or punctuation expands. Anywhere removes that protection and lets a trigger fire in the middle of a word."
              control={
                <SegmentedControl<BoundaryMode>
                  label="Trigger position"
                  value={settings.boundaryMode}
                  options={BOUNDARY_OPTIONS}
                  onChange={(boundaryMode) => apply({ boundaryMode })}
                />
              }
            />

            <SettingsRow
              label="Keep the ending character"
              hint="Whether the key that completed the trigger is sent again after the snippet. A trigger only fires once you finish it with a space, a full stop, Tab or Enter, and Ampello swallows that key while it works. With this on it is sent afterwards, so “:email.” keeps its full stop and “:email” followed by Enter still sends the message."
              control={
                <Switch
                  label="Keep the ending character"
                  checked={settings.preserveBoundaryChar}
                  onChange={(preserveBoundaryChar) => apply({ preserveBoundaryChar })}
                />
              }
            />

            <SettingsRow
              label="Insertion method"
              hint="How a snippet reaches the application you are typing in. Auto sends short single-line snippets as keystrokes and uses the clipboard for anything longer, multi-line or indented. Leave this selected unless something is wrong. Paste always uses the clipboard: instant and exact, but the application has to accept Ctrl+V. Type always sends keystrokes and never touches the clipboard, for applications that refuse a paste; it is slower, and an application that ignores character-level tabs will lose the indentation. A long insertion can be stopped part-way with Escape."
              control={
                <SegmentedControl<InjectionMode>
                  label="Insertion method"
                  value={settings.injectionMode}
                  options={INJECTION_OPTIONS}
                  onChange={(injectionMode) => apply({ injectionMode })}
                />
              }
            />

            <SettingsRow
              label="Typing speed"
              hint="How fast a snippet is typed, when Ampello types it rather than pasting it. It goes in one character at a time, the way you would type it yourself: Fast is around 250 characters a second, Balanced around 150, and Careful around 60, which is about as quick as a fast typist. Slower is not only calmer to watch, it is more reliable. An application that cannot keep up queues the surplus somewhere Ampello can no longer reach, which is what makes a long insertion carry on typing after you have pressed Escape. Pasting is unaffected."
              control={
                <SegmentedControl<TypingSpeed>
                  label="Typing speed"
                  value={settings.typingSpeed}
                  options={TYPING_OPTIONS}
                  onChange={(typingSpeed) => apply({ typingSpeed })}
                />
              }
            />

            <SettingsRow
              label="Restore the clipboard"
              hint="Pasting a snippet means putting it on the clipboard first, which would otherwise destroy whatever you had copied. With this on, Ampello reads the clipboard out beforehand (text, images, copied files) and writes it back once the paste has landed. If it finds something it cannot copy faithfully, it types the snippet instead rather than overwrite it."
              control={
                <Switch
                  label="Restore the clipboard"
                  checked={settings.restoreClipboard}
                  onChange={(restoreClipboard) => apply({ restoreClipboard })}
                />
              }
            />

            <SettingsRow
              label="Attachment delay"
              hint="How long Ampello waits after handing files to an application before sending it anything else. There is no way to ask an application whether it has finished taking an attachment - it reads the clipboard in its own time, and a chat box may start an upload before it will accept anything more - so this is a guess, and the right value depends on the application and the machine. Too short and the message text arrives before the files; too long and every snippet with a file feels slow."
              control={
                <div className="flex items-center gap-2">
                  <input
                    type="number"
                    min={MIN_SETTLE_MS}
                    max={MAX_SETTLE_MS}
                    step={50}
                    value={settings.attachmentSettleMs}
                    onChange={(event) => {
                      const value = Number(event.target.value);
                      if (!Number.isFinite(value)) return;
                      apply({
                        attachmentSettleMs: Math.min(
                          MAX_SETTLE_MS,
                          Math.max(MIN_SETTLE_MS, Math.round(value)),
                        ),
                      });
                    }}
                    className="h-8 w-[86px] rounded-[8px] border border-border bg-surface px-2 text-right text-[13px] tabular-nums text-primary focus:border-accent focus:outline-none"
                    aria-label="Attachment delay in milliseconds"
                  />
                  <span className="text-[12.5px] text-muted">ms</span>
                </div>
              }
            />
          </SettingsSection>
          <SettingsSection title="Clipboard shortcut">
            <SettingsRow
              label="Enabled"
              hint="A second key combination that inserts whatever is on your clipboard, alongside the usual Ctrl+V rather than replacing it. Switch it off and the combination goes straight back to the application you are working in. It is also released while snippet expansion is paused, because pausing Ampello pauses all of it."
              control={
                <Switch
                  label="Clipboard shortcut"
                  checked={settings.clipboardShortcutEnabled}
                  onChange={(clipboardShortcutEnabled) =>
                    apply({ clipboardShortcutEnabled })
                  }
                />
              }
            />

            <SettingsRow
              label="Combination"
              hint="The combination that inserts the clipboard. Ctrl Shift V by default, which few applications use for anything you would miss. It cannot be the same as the global shortcut, and if another application has already claimed it Ampello will say so rather than failing quietly."
              control={
                <ShortcutRecorder
                  value={settings.clipboardShortcut}
                  onChange={(clipboardShortcut) => apply({ clipboardShortcut })}
                />
              }
            />

            <SettingsRow
              label="Method"
              hint="What the combination does. Type sends the clipboard as individual keystrokes at the typing speed set above, which is how text gets into an application that refuses a paste or reformats one; a long clipboard takes a while, and Escape stops it part-way. Paste sends a plain Ctrl+V, which is instant and exact. If the clipboard holds an image or a list of files there is nothing to type, so Ampello pastes either way."
              control={
                <SegmentedControl<ClipboardMode>
                  label="Method"
                  value={settings.clipboardMode}
                  options={CLIPBOARD_OPTIONS}
                  onChange={(clipboardMode) => apply({ clipboardMode })}
                />
              }
            />
          </SettingsSection>
          <SettingsSection title="General">
            <SettingsRow
              label="Theme"
              hint="Light, dark, or follow the Windows app theme and change with it as Windows does."
              control={
                <SegmentedControl<Appearance>
                  label="Theme"
                  value={settings.appearance}
                  options={APPEARANCE_OPTIONS}
                  onChange={(value) => void setAppearance(value).catch(reportError)}
                />
              }
            />

            <SettingsRow
              label="Launch at startup"
              hint="Registers Ampello with Windows so it starts when you sign in. A startup launch goes straight to the notification area without opening a window."
              control={
                <Switch
                  label="Launch at startup"
                  checked={settings.launchAtStartup}
                  onChange={(launchAtStartup) => apply({ launchAtStartup })}
                />
              }
            />

            <SettingsRow
              label="Close to the tray"
              hint="What the window\u2019s close button does. On, closing the window leaves Ampello running in the notification area and snippets keep expanding. Off, closing the window quits Ampello entirely and expansion stops until you start it again."
              control={
                <Switch
                  label="Close to the tray"
                  checked={settings.closeToTray}
                  onChange={(closeToTray) => apply({ closeToTray })}
                />
              }
            />

            <SettingsRow
              label="Global shortcut"
              hint="A key combination that brings Ampello forward from any application, and puts it away again if it is already in front. If another application has already claimed the combination Ampello will say so rather than failing quietly."
              control={
                <ShortcutRecorder
                  value={settings.globalShortcut}
                  onChange={(globalShortcut) => apply({ globalShortcut })}
                />
              }
            />

            {shortcutProblem ? (
              <Notice tone="warning">
                {shortcutProblem} It is saved, but will not respond until you pick
                another.
              </Notice>
            ) : null}
          </SettingsSection>
          <SettingsSection title="Keyboard">
            <SettingsBlock className="grid grid-cols-2 gap-x-10 gap-y-2.5">
              {SHORTCUTS.map(([keys, description]) => (
                <div
                  key={description}
                  className="flex items-baseline justify-between gap-3"
                >
                  <span className="truncate text-[12.5px] text-secondary">
                    {description}
                  </span>
                  <Kbd keys={keys} className="shrink-0" />
                </div>
              ))}
            </SettingsBlock>
          </SettingsSection>
          <SettingsSection title="Data">
            {info?.recoveredFrom ? (
              <Notice tone="danger">
                Ampello could not read your previous library and started a fresh one.
                Nothing was deleted. The old file is still on disk, and a backup can
                be imported below.
                <Path>{info.recoveredFrom}</Path>
              </Notice>
            ) : null}

            <SettingsRow
              label="Library"
              hint="How much Ampello is storing, and how large the database file has become. Everything lives in a single SQLite file on this machine and is never uploaded."
              control={
                <span className="text-[12.5px] tabular-nums text-secondary">
                  {info
                    ? `${info.snippetCount.toLocaleString()} snippet${info.snippetCount === 1 ? "" : "s"} · ${formatBytes(info.sizeBytes)}`
                    : "…"}
                </span>
              }
            />

            {library?.problem ? (
              <Notice tone="danger">
                The shared library could not be opened, so Ampello is using this
                account&rsquo;s own library instead. Nothing was lost.
                <Path>{library.problem}</Path>
              </Notice>
            ) : null}

            <SettingsRow
              label="Location"
              hint="Where this account keeps its snippets. A personal library is private to your Windows account. A shared folder lets everyone who points at it use the same snippets and attachments - useful on a family or office machine. Sharing is never automatic: each account has to choose the folder itself. Keep it on a local disk; a network share will not work reliably."
              control={
                <div className="flex items-center gap-2">
                  <span className="text-[12.5px] text-secondary">
                    {library ? (library.shared ? "Shared" : "Personal") : "…"}
                  </span>
                  {library?.shared ? (
                    <Button
                      size="sm"
                      disabled={busy !== null}
                      onClick={() => void revertLibrary()}
                    >
                      Use personal
                    </Button>
                  ) : null}
                  <Button
                    size="sm"
                    disabled={busy !== null}
                    onClick={() => void pickLibrary()}
                  >
                    {library?.shared ? "Change…" : "Share…"}
                  </Button>
                </div>
              }
            />

            {pendingRestart ? (
              <SettingsBlock>
                <div className="flex flex-wrap items-center gap-3">
                  <p className="flex-1 text-[12.5px] text-secondary">
                    Ampello will use the new library location when it restarts.
                    Snippets are not copied between libraries - use Export and
                    Import to move them.
                  </p>
                  <Button size="sm" variant="primary" onClick={() => void ipc.restartApp()}>
                    Restart now
                  </Button>
                </div>
              </SettingsBlock>
            ) : null}

            <SettingsRow
              label="Export"
              hint="Writes your whole library to one file: every trigger, title and body, which collection each belongs to, and whether it is enabled or a favourite. Content is stored byte for byte. YAML is readable and safe to edit by hand; JSON is there for other tools."
              control={
                <div className="flex items-center gap-2">
                  <Button size="sm" disabled={busy !== null} onClick={() => runExport("yaml")}>
                    YAML
                  </Button>
                  <Button size="sm" disabled={busy !== null} onClick={() => runExport("json")}>
                    JSON
                  </Button>
                </div>
              }
            />

            <SettingsRow
              label="Import"
              hint="Reads a Ampello backup and adds it to your library. Nothing you already have is touched: a snippet whose trigger you are already using is skipped and reported, so importing can never overwrite your own work."
              control={
                <Button size="sm" disabled={busy !== null} onClick={runImport}>
                  {busy === "import" ? "Importing…" : "Import…"}
                </Button>
              }
            />

            {report ? (
              <SettingsBlock>
                <ImportSummary report={report} />
              </SettingsBlock>
            ) : null}

            <SettingsBlock>
              <details className="group">
                <summary className="cursor-pointer list-none text-[12.5px] text-secondary transition-colors duration-150 hover:text-primary">
                  <span className="underline decoration-border underline-offset-4">
                    Where things are kept
                  </span>
                </summary>
                <div className="motion-fade mt-2.5 space-y-2">
                  <p className="text-[12px] leading-relaxed text-muted">
                    Local-first: one SQLite file on this machine, never uploaded. The
                    log records what Ampello did, never what you typed.
                  </p>
                  {info ? (
                    <Path label={library?.shared ? "Shared library" : "Library"}>
                      {info.path}
                    </Path>
                  ) : null}
                  {diag?.logDirectory ? <Path label="Log">{diag.logDirectory}</Path> : null}
                </div>
              </details>
            </SettingsBlock>
          </SettingsSection>
          <SettingsSection title="About">
            <SettingsRow
              label="Version"
              control={<span className="text-[12.5px] tabular-nums text-secondary">0.1.0</span>}
            />
          </SettingsSection>
        </div>
      </main>
    </>
  );
}

function Notice({
  tone,
  children,
}: {
  tone: "danger" | "warning";
  children: ReactNode;
}) {
  return (
    <div
      className={cn(
        "flex items-start gap-2 px-4 py-3",
        tone === "danger" ? "bg-danger-soft text-danger" : "bg-warning-soft text-warning",
      )}
    >
      <TriangleAlert size={15} strokeWidth={1.75} className="mt-px shrink-0" />
      <div className="min-w-0 text-[12.5px] leading-relaxed">{children}</div>
    </div>
  );
}

function Path({ label, children }: { label?: string; children: React.ReactNode }) {
  return (
    <p className="select-text break-all rounded-[5px] border border-border bg-surface-2 px-2.5 py-1.5 font-mono text-[11.5px] text-secondary">
      {label ? <span className="mr-2 not-italic opacity-60">{label}</span> : null}
      {children}
    </p>
  );
}

function ImportSummary({ report }: { report: ImportReport }) {
  const parts = [
    `${report.added} added`,
    report.replaced > 0 ? `${report.replaced} replaced` : null,
    report.skipped > 0 ? `${report.skipped} left alone` : null,
    report.collectionsCreated > 0
      ? `${report.collectionsCreated} collection${report.collectionsCreated === 1 ? "" : "s"} created`
      : null,
  ].filter(Boolean);

  return (
    <div className="rounded-[8px] border border-border bg-surface-2 px-3 py-2.5">
      <p className="text-[12.5px] text-primary">{parts.join(" · ")}</p>
      {report.problems.length > 0 ? (
        <ul className="mt-1.5 space-y-0.5">
          {report.problems.slice(0, 6).map((problem) => (
            <li key={problem} className="text-[12px] leading-relaxed text-warning">
              {problem}
            </li>
          ))}
          {report.problems.length > 6 ? (
            <li className="text-[12px] text-muted">
              …and {report.problems.length - 6} more.
            </li>
          ) : null}
        </ul>
      ) : null}
    </div>
  );
}

function EngineBanner({
  status,
  restarting,
  onRestart,
}: {
  status: EngineStatus | null;
  restarting: boolean;
  onRestart: () => void;
}) {
  if (!status) {
    return (
      <p className="text-[12.5px] text-muted">Checking the expansion engine…</p>
    );
  }

  const broken = Boolean(status.error) || !status.running;

  return (
    <div
      className={cn(
        "-mx-4 -my-3.5 px-4 py-3.5",
        broken ? "bg-danger-soft" : "bg-surface-2",
      )}
    >
      <div className="flex items-center gap-2.5">
        {broken ? (
          <TriangleAlert size={15} strokeWidth={1.75} className="shrink-0 text-danger" />
        ) : (
          <span
            aria-hidden="true"
            className="h-[7px] w-[7px] shrink-0 rounded-full"
            style={{
              backgroundColor: status.enabled ? "var(--success)" : "var(--text-muted)",
            }}
          />
        )}

        <p
          className={cn(
            "min-w-0 flex-1 text-[12.5px]",
            broken ? "text-danger" : "text-secondary",
          )}
        >
          {broken
            ? (status.error ?? "The expansion engine is not running.")
            : `${status.enabled ? "Listening" : "Paused"} · ${status.triggerCount.toLocaleString()} trigger${status.triggerCount === 1 ? "" : "s"} · ${status.keystrokesSeen.toLocaleString()} keystrokes · ${status.expansions.toLocaleString()} expansion${status.expansions === 1 ? "" : "s"}`}
        </p>

        <Button size="sm" disabled={restarting} onClick={onRestart} className="shrink-0">
          {restarting ? "Restarting…" : "Restart"}
        </Button>
      </div>

      {status.lastExpansionError ? (
        <p className="mt-2 text-[12px] leading-relaxed text-warning">
          Last expansion failed: {status.lastExpansionError}
        </p>
      ) : null}
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
