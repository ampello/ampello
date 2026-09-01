// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { ArrowLeft, Star } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { IconButton } from "@/components/ui/IconButton";
import { Input } from "@/components/ui/Input";
import { Spinner } from "@/components/ui/Spinner";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { AttachmentList } from "@/components/snippets/AttachmentList";
import { CodeEditor } from "@/editor/CodeEditor";
import type { EditorHandle } from "@/editor/CodeEditor";
import * as ipc from "@/lib/ipc";
import { cn } from "@/lib/cn";
import { useDataStore } from "@/stores/dataStore";
import { useUiStore } from "@/stores/uiStore";
import { reportError, useToastStore } from "@/stores/toastStore";
import type { Snippet } from "@/lib/types";

interface Loaded {
  snippet: Snippet | null;
  trigger: string;
  content: string;
  favorite: boolean;
  categoryId: string | null;
}

export function EditorView({ id }: { id: string }) {
  const isNew = id === "new";
  const closeEditor = useUiStore((s) => s.closeEditor);
  const draftCategoryId = useUiStore((s) => s.draftCategoryId);
  const categories = useDataStore((s) => s.categories);
  const createSnippet = useDataStore((s) => s.createSnippet);
  const saveSnippet = useDataStore((s) => s.saveSnippet);
  const pushToast = useToastStore((s) => s.push);

  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  const [trigger, setTrigger] = useState("");
  const [favorite, setFavorite] = useState(false);
  const [categoryId, setCategoryId] = useState<string | null>(null);

  const [contentDirty, setContentDirty] = useState(false);
  const [fieldsDirty, setFieldsDirty] = useState(false);
  const [triggerProblem, setTriggerProblem] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [confirmDiscard, setConfirmDiscard] = useState(false);

  const editorRef = useRef<EditorHandle | null>(null);
  const dirty = contentDirty || fieldsDirty;

  useEffect(() => {
    let cancelled = false;
    const run = async () => {
      if (isNew) {
        if (!cancelled) {
          setCategoryId(draftCategoryId);
          setLoaded({
            snippet: null,
            trigger: "",
            content: "",
            favorite: false,
            categoryId: draftCategoryId,
          });
        }
        return;
      }
      try {
        const snippet = await ipc.getSnippet(id);
        if (cancelled) return;
        setLoaded({
          snippet,
          trigger: snippet.trigger,
          content: snippet.content,
          favorite: snippet.favorite,
          categoryId: snippet.categoryId,
        });
        setTrigger(snippet.trigger);
        setFavorite(snippet.favorite);
        setCategoryId(snippet.categoryId);
      } catch (error) {
        if (!cancelled) {
          setLoadError(error instanceof Error ? error.message : String(error));
        }
      }
    };
    void run();
    return () => {
      cancelled = true;
    };
  }, [id, isNew, draftCategoryId]);

  useEffect(() => {
    const value = trigger.trim();
    if (!value) {
      setTriggerProblem(null);
      return;
    }
    let cancelled = false;
    const timer = setTimeout(() => {
      ipc
        .triggerAvailable(value, isNew ? null : id)
        .then((available) => {
          if (!cancelled) {
            setTriggerProblem(available ? null : "Another snippet already uses this trigger.");
          }
        })
        .catch((error) => {
          if (!cancelled) {
            setTriggerProblem(error instanceof Error ? error.message : String(error));
          }
        });
    }, 200);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [trigger, id, isNew]);

  const markFields = () => setFieldsDirty(true);

  const save = async () => {
    if (saving) return;
    const cleanTrigger = trigger.trim();
    if (!cleanTrigger) {
      setTriggerProblem("A trigger is required.");
      return;
    }
    if (triggerProblem) return;

    const content = editorRef.current?.getValue() ?? loaded?.content ?? "";
    setSaving(true);
    try {
      if (isNew) {
        const created = await createSnippet({
          trigger: cleanTrigger,
          content,
          categoryId,
        });
        if (favorite) await saveSnippet(created.id, { favorite: true });
      } else {
        await saveSnippet(id, {
          trigger: cleanTrigger,
          content,
          favorite,
          categoryId,
        });
      }
      setContentDirty(false);
      setFieldsDirty(false);
      pushToast("info", isNew ? `Created ${cleanTrigger}` : `Saved ${cleanTrigger}`);
      closeEditor();
    } catch (error) {
      reportError(error);
    } finally {
      setSaving(false);
    }
  };

  const requestClose = () => {
    if (dirty) setConfirmDiscard(true);
    else closeEditor();
  };

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return;
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
        event.preventDefault();
        void save();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  if (loadError) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center gap-4">
        <p className="text-[13px] text-secondary">{loadError}</p>
        <Button onClick={closeEditor}>Back to snippets</Button>
      </div>
    );
  }

  if (!loaded) {
    return (
      <div className="flex flex-1 items-center justify-center">
        <Spinner />
      </div>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header
        data-tauri-drag-region
        className="flex h-12 shrink-0 items-center gap-3 border-b border-border bg-bg pl-3 pr-[158px]"
      >
        <IconButton label="Back to snippets" onClick={requestClose}>
          <ArrowLeft size={16} strokeWidth={1.75} />
        </IconButton>
        <h1
          data-tauri-drag-region
          className="text-[13.5px] font-semibold text-primary"
        >
          {isNew ? "New Snippet" : "Edit Snippet"}
        </h1>
        {dirty ? (
          <span className="text-[12px] text-muted">Unsaved changes</span>
        ) : null}

        <div data-tauri-drag-region className="flex-1" />

        <IconButton
          label={favorite ? "Remove from favorites" : "Add to favorites"}
          aria-pressed={favorite}
          onClick={() => {
            setFavorite((value) => !value);
            markFields();
          }}
          className={cn(favorite && "text-accent hover:text-accent")}
        >
          <Star size={15} strokeWidth={1.75} fill={favorite ? "currentColor" : "none"} />
        </IconButton>
        <Button onClick={requestClose}>Cancel</Button>
        <Button variant="primary" onClick={() => void save()} disabled={saving}>
          {saving ? "Saving…" : "Save"}
        </Button>
      </header>

      <div className="flex min-h-0 flex-1 flex-col px-6 pb-4 pt-5">
        <div className="mb-4 flex flex-wrap items-start gap-4">
          <Field label="Trigger" className="min-w-[240px] flex-1" error={triggerProblem}>
            <Input
              mono
              value={trigger}
              placeholder=":email"
              autoFocus={isNew}
              invalid={Boolean(triggerProblem)}
              onChange={(event) => {
                setTrigger(event.target.value);
                markFields();
              }}
            />
          </Field>

          <Field label="Collection" className="w-[190px]">
            <select
              value={categoryId ?? ""}
              onChange={(event) => {
                setCategoryId(event.target.value || null);
                markFields();
              }}
              className="h-8 w-full rounded-[6px] border border-border bg-surface px-2 text-[13px] text-primary focus:border-accent focus:outline-none"
            >
              <option value="">None</option>
              {categories.map((category) => (
                <option key={category.id} value={category.id}>
                  {category.name}
                </option>
              ))}
            </select>
          </Field>
        </div>

        <CodeEditor
          initialValue={loaded.content}
          handleRef={editorRef}
          onDirtyChange={setContentDirty}
          onSave={() => void save()}
          onEscape={requestClose}
        />
        <AttachmentList
          snippet={loaded.snippet}
          onChange={(updated) =>
            setLoaded((current) => (current ? { ...current, snippet: updated } : current))
          }
        />
      </div>

      {confirmDiscard ? (
        <ConfirmDialog
          title="Discard changes?"
          description="This snippet has unsaved changes. Closing now loses them."
          confirmLabel="Discard"
          cancelLabel="Keep editing"
          danger
          onCancel={() => setConfirmDiscard(false)}
          onConfirm={() => {
            setConfirmDiscard(false);
            closeEditor();
          }}
        />
      ) : null}
    </div>
  );
}

function Field({
  label,
  error,
  className,
  children,
}: {
  label: string;
  error?: string | null;
  className?: string;
  children: ReactNode;
}) {
  return (
    <div className={className}>
      <label className="mb-1 block text-[11px] font-medium uppercase tracking-[0.06em] text-muted">
        {label}
      </label>
      {children}
      {error ? <p className="mt-1 text-[12px] text-danger">{error}</p> : null}
    </div>
  );
}
