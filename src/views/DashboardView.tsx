// SPDX-License-Identifier: GPL-3.0-or-later
import { useCallback, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { RotateCw, TriangleAlert } from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { Button } from "@/components/ui/Button";
import { Kbd } from "@/components/ui/Kbd";
import { Switch } from "@/components/ui/Switch";
import { humanise } from "@/components/ui/ShortcutRecorder";
import * as ipc from "@/lib/ipc";
import { cn } from "@/lib/cn";
import type { EngineStatus, SnippetSummary } from "@/lib/types";
import { useDataStore } from "@/stores/dataStore";
import { useSettingsStore } from "@/stores/settingsStore";
import { reportError } from "@/stores/toastStore";
import { useUiStore } from "@/stores/uiStore";

export function DashboardView() {
  const snippets = useDataStore((s) => s.snippets);
  const setScope = useUiStore((s) => s.setScope);
  const openEditor = useUiStore((s) => s.openEditor);

  const settings = useSettingsStore((s) => s.settings);
  const patch = useSettingsStore((s) => s.patch);

  const [engine, setEngine] = useState<EngineStatus | null>(null);
  const [restarting, setRestarting] = useState(false);

  const readEngine = useCallback(() => {
    ipc.engineStatus().then(setEngine).catch(() => setEngine(null));
  }, []);

  useEffect(() => {
    readEngine();

    const timer = window.setInterval(readEngine, 4000);
    return () => window.clearInterval(timer);
  }, [readEngine]);

  const mostUsed = useMemo(
    () =>
      snippets
        .filter((s) => s.usageCount > 0)
        .sort((a, b) => b.usageCount - a.usageCount)
        .slice(0, 6),
    [snippets],
  );

  const recentlyEdited = useMemo(
    () => [...snippets].sort((a, b) => b.updatedAt - a.updatedAt).slice(0, 6),
    [snippets],
  );

  const favorites = snippets.filter((s) => s.favorite).length;
  const disabled = snippets.filter((s) => !s.enabled).length;

  const toggleExpansion = (enabled: boolean) => {
    void patch({ expansionEnabled: enabled }).then(readEngine).catch(reportError);
  };

  const restart = () => {
    setRestarting(true);
    ipc
      .restartEngine()
      .then(setEngine)
      .catch(reportError)
      .finally(() => setRestarting(false));
  };

  return (
    <>
      <TopBar title="Home" />
      <main className="flex-1 overflow-y-auto">
        <div className="mx-auto w-full max-w-[840px] px-8 py-9">
          <StatusCard
            engine={engine}
            enabled={settings.expansionEnabled}
            shortcut={settings.globalShortcut}
            restarting={restarting}
            onToggle={toggleExpansion}
            onRestart={restart}
            onBrowse={() => setScope({ kind: "all" })}
          />

          {snippets.length === 0 ? (
            <GettingStarted onCreate={() => openEditor(null)} />
          ) : (
            <>
              <div className="mt-7 grid gap-5 md:grid-cols-2">
                <SnippetCard
                  title="Most used"
                  empty="Nothing has fired yet. Type one of your triggers anywhere."
                  snippets={mostUsed}
                  meta={(s) => `${s.usageCount.toLocaleString()}×`}
                  onOpen={openEditor}
                />
                <SnippetCard
                  title="Recently edited"
                  empty="Nothing here yet."
                  snippets={recentlyEdited}
                  meta={(s) => ago(s.updatedAt)}
                  onOpen={openEditor}
                />
              </div>

              <p className="mt-6 px-1 text-[12.5px] text-muted">
                {count(snippets.length, "snippet")}
                {favorites > 0 ? ` · ${favorites} favorite${favorites === 1 ? "" : "s"}` : ""}
                {disabled > 0 ? ` · ${disabled} disabled` : ""}
              </p>
            </>
          )}
        </div>
      </main>
    </>
  );
}

function StatusCard({
  engine,
  enabled,
  shortcut,
  restarting,
  onToggle,
  onRestart,
  onBrowse,
}: {
  engine: EngineStatus | null;
  enabled: boolean;
  shortcut: string;
  restarting: boolean;
  onToggle: (enabled: boolean) => void;
  onRestart: () => void;
  onBrowse: () => void;
}) {
  const broken = engine !== null && (Boolean(engine.error) || !engine.running);

  let headline: string;
  let detail: string;
  if (engine === null) {
    headline = "Checking the expansion engine…";
    detail = "";
  } else if (broken) {
    headline = "Expansion is not running";
    detail = engine.error ?? "Windows is not handing Ampello any keystrokes.";
  } else if (!enabled) {
    headline = "Expansion is paused";
    detail = "Ampello is still running; nothing is being watched for or replaced.";
  } else {
    headline = "Expansion is on";
    detail = `Watching for ${count(engine.triggerCount, "trigger")}.`;
  }

  return (
    <section className="divide-y divide-border overflow-hidden rounded-[12px] border border-border bg-surface">
      <div
        className={cn(
          "flex items-center gap-3 px-4 py-3.5",
          broken && "bg-danger-soft",
        )}
      >
        {broken ? (
          <TriangleAlert size={15} strokeWidth={1.75} className="shrink-0 text-danger" />
        ) : (
          <span
            aria-hidden="true"
            className="ml-[3px] mr-[2px] h-[7px] w-[7px] shrink-0 rounded-full transition-colors duration-200"
            style={{
              backgroundColor:
                engine && enabled ? "var(--success)" : "var(--text-muted)",
            }}
          />
        )}

        <div className="min-w-0 flex-1">
          <p className={cn("text-[13px]", broken ? "text-danger" : "text-primary")}>
            {headline}
          </p>
          {detail ? (
            <p className="mt-0.5 truncate text-[12px] text-muted">{detail}</p>
          ) : null}
        </div>

        {broken ? (
          <Button size="sm" disabled={restarting} onClick={onRestart} className="shrink-0 gap-1.5">
            <RotateCw size={13} strokeWidth={1.75} />
            {restarting ? "Restarting…" : "Restart"}
          </Button>
        ) : (
          <Switch
            label="Expansion enabled"
            checked={enabled}
            onChange={onToggle}
            disabled={engine === null}
          />
        )}
      </div>

      <div className="flex items-center justify-between gap-4 px-4 py-3">
        <p className="flex min-w-0 items-center gap-2 text-[12.5px] text-secondary">
          <Kbd keys={humanise(shortcut)} />
          <span className="truncate">brings this window back from anywhere</span>
        </p>
        <button
          type="button"
          onClick={onBrowse}
          className="shrink-0 text-[12.5px] font-medium text-accent hover:underline"
        >
          Browse all snippets
        </button>
      </div>
    </section>
  );
}

function SnippetCard({
  title,
  empty,
  snippets,
  meta,
  onOpen,
}: {
  title: string;
  empty: string;
  snippets: SnippetSummary[];
  meta: (snippet: SnippetSummary) => string;
  onOpen: (id: string) => void;
}) {
  return (
    <section>
      <h3 className="mb-2 px-1 text-[11px] font-medium uppercase tracking-[0.06em] text-muted">
        {title}
      </h3>
      <div className="overflow-hidden rounded-[12px] border border-border bg-surface">
        {snippets.length === 0 ? (
          <p className="px-4 py-3.5 text-[12.5px] leading-relaxed text-muted">{empty}</p>
        ) : (
          <ul className="divide-y divide-border">
            {snippets.map((snippet) => (
              <li key={snippet.id}>
                <button
                  type="button"
                  onClick={() => onOpen(snippet.id)}
                  className={cn(
                    "flex w-full items-baseline gap-2.5 px-4 py-2.5 text-left",
                    "transition-colors duration-150 hover:bg-surface-2",
                  )}
                >
                  <span
                    className={cn(
                      "shrink-0 font-mono text-[12.5px]",
                      snippet.enabled ? "text-accent" : "text-muted line-through",
                    )}
                  >
                    {snippet.trigger}
                  </span>
                  <span className="min-w-0 flex-1 truncate text-[13px] text-secondary">
                    {snippet.preview}
                  </span>
                  <span className="shrink-0 text-[11.5px] tabular-nums text-muted">
                    {meta(snippet)}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}

function GettingStarted({ onCreate }: { onCreate: () => void }) {
  return (
    <section className="mt-7">
      <h3 className="mb-2 px-1 text-[11px] font-medium uppercase tracking-[0.06em] text-muted">
        Getting started
      </h3>
      <div className="divide-y divide-border overflow-hidden rounded-[12px] border border-border bg-surface">
        <Step
          number={1}
          title="Write a snippet"
          body={
            <>
              A trigger such as <code className="font-mono text-accent">:sig</code> and the
              text it stands for. The text can be a word or a thousand lines; Ampello does not
              care which.
            </>
          }
        />
        <Step
          number={2}
          title="Type the trigger anywhere"
          body="In any application on this computer. Ampello replaces it the moment you finish typing it."
        />
        <Step
          number={3}
          title="Leave it running"
          body="Closing this window keeps Ampello in the notification area, still listening."
        />
        <div className="px-4 py-3.5">
          <Button onClick={onCreate}>Create your first snippet</Button>
        </div>
      </div>
    </section>
  );
}

function Step({
  number,
  title,
  body,
}: {
  number: number;
  title: string;
  body: ReactNode;
}) {
  return (
    <div className="flex gap-3 px-4 py-3.5">
      <span
        aria-hidden="true"
        className={cn(
          "mt-[1px] flex h-[19px] w-[19px] shrink-0 items-center justify-center rounded-full",
          "bg-accent-soft text-[11px] font-medium tabular-nums text-accent",
        )}
      >
        {number}
      </span>
      <div className="min-w-0">
        <p className="text-[13px] text-primary">{title}</p>
        <p className="mt-0.5 text-[12.5px] leading-relaxed text-muted">{body}</p>
      </div>
    </div>
  );
}

function count(value: number, noun: string): string {
  return `${value.toLocaleString()} ${noun}${value === 1 ? "" : "s"}`;
}

function ago(timestamp: number): string {
  const seconds = Math.max(0, Math.round((Date.now() - timestamp) / 1000));
  if (seconds < 60) return "just now";
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.round(hours / 24);
  if (days < 7) return `${days}d ago`;
  const weeks = Math.round(days / 7);
  if (weeks < 5) return `${weeks}w ago`;
  return new Date(timestamp).toLocaleDateString(undefined, {
    day: "numeric",
    month: "short",
  });
}
