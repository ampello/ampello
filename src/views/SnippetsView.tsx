// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useMemo, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import { TopBar } from "@/components/layout/TopBar";
import { EmptyState } from "@/components/ui/EmptyState";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { Spinner } from "@/components/ui/Spinner";
import { SearchField } from "@/components/snippets/SearchField";
import { SnippetList } from "@/components/snippets/SnippetList";
import { EditorView } from "@/views/EditorView";
import { Button } from "@/components/ui/Button";
import { useDataStore } from "@/stores/dataStore";
import { useUiStore } from "@/stores/uiStore";
import { reportError } from "@/stores/toastStore";
import type { SnippetSummary } from "@/lib/types";

export function SnippetsView() {
  const scope = useUiStore((s) => s.scope);
  const openEditor = useUiStore((s) => s.openEditor);
  const editingId = useUiStore((s) => s.editingId);
  const closeEditor = useUiStore((s) => s.closeEditor);

  const snippets = useDataStore((s) => s.snippets);
  const categories = useDataStore((s) => s.categories);
  const results = useDataStore((s) => s.results);
  const query = useDataStore((s) => s.query);
  const searching = useDataStore((s) => s.searching);
  const setQuery = useDataStore((s) => s.setQuery);
  const setFavorite = useDataStore((s) => s.setFavorite);
  const setEnabled = useDataStore((s) => s.setEnabled);
  const removeSnippet = useDataStore((s) => s.removeSnippet);

  const [pendingDelete, setPendingDelete] = useState<SnippetSummary | null>(null);

  // A row is selected precisely when the pane is showing it. "new" is a
  // draft that has no row yet.
  const selectedId = editingId === "new" ? null : editingId;
  const setSelectedId = (id: string | null) => (id ? openEditor(id) : closeEditor());

  const scopeTitle =
    scope.kind === "all"
      ? "All Snippets"
      : scope.kind === "favorites"
        ? "Favorites"
        : (categories.find((c) => c.id === scope.id)?.name ?? "Collection");

  const visible = useMemo(() => {
    const base = results ?? snippets;
    if (scope.kind === "favorites") return base.filter((s) => s.favorite);
    if (scope.kind === "category") return base.filter((s) => s.categoryId === scope.id);
    return base;
  }, [results, snippets, scope]);

  useEffect(() => {
    if (selectedId && !visible.some((s) => s.id === selectedId)) setSelectedId(null);
  }, [visible, selectedId]);

  const move = (delta: number) => {
    if (visible.length === 0) return;
    const current = visible.findIndex((s) => s.id === selectedId);
    const next =
      current < 0
        ? delta > 0
          ? 0
          : visible.length - 1
        : Math.min(visible.length - 1, Math.max(0, current + delta));
    setSelectedId(visible[next]?.id ?? null);
  };

  const onKeyDown = (event: ReactKeyboardEvent) => {
    if (pendingDelete || event.defaultPrevented) return;

    if (event.key === "ArrowDown") {
      event.preventDefault();
      move(1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      move(-1);
    } else if (event.key === "Delete" && selectedId && !isTyping(event.target)) {
      const snippet = visible.find((s) => s.id === selectedId);
      if (snippet) {
        event.preventDefault();
        setPendingDelete(snippet);
      }
    }
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col" onKeyDown={onKeyDown}>
      <TopBar
        title={scopeTitle}
        meta={visible.length > 0 ? String(visible.length) : undefined}
        center={<SearchField value={query} onChange={setQuery} />}
      />

      <div className="flex min-h-0 flex-1">
        <div className="min-h-0 min-w-0 flex-1">
        {searching && visible.length === 0 ? (
          <div className="flex h-full items-center justify-center">
            <Spinner />
          </div>
        ) : visible.length > 0 ? (
          <SnippetList
            snippets={visible}
            categories={categories}
            selectedId={selectedId}
            onSelect={setSelectedId}
            onOpen={openEditor}
            onToggleFavorite={(snippet) =>
              void setFavorite(snippet.id, !snippet.favorite).catch(reportError)
            }
            onToggleEnabled={(snippet) =>
              void setEnabled(snippet.id, !snippet.enabled).catch(reportError)
            }
            onDelete={setPendingDelete}
          />
        ) : query.trim() ? (
          <EmptyState
            title={`No snippets match “${query.trim()}”`}
            description="Search looks at triggers, titles, collections and the full text of every snippet."
          />
        ) : scope.kind === "favorites" ? (
          <EmptyState
            title="No favorites yet"
            description="Star a snippet to keep it here."
          />
        ) : snippets.length === 0 ? (
          <EmptyState
            title="No snippets yet"
            description="A snippet is a trigger and the text it becomes. Type the trigger anywhere and Ampello puts the text there. New Snippet is in the sidebar."
          />
        ) : (
          <EmptyState description="A snippet created while this collection is open is added to it. An existing snippet can be moved in from its editor." />
        )}
        </div>

        {/* The detail pane: the selected snippet stays beside the list rather
            than replacing it, so moving between snippets costs one click. */}
        <aside className="flex w-[420px] shrink-0 flex-col border-l border-border bg-bg">
          {editingId ? (
            <EditorView key={editingId} id={editingId} inline />
          ) : (
            <div className="flex flex-1 flex-col items-center justify-center gap-3 px-6 text-center">
              <p className="text-[13px] font-medium text-primary">No snippet selected</p>
              <p className="text-[12.5px] text-secondary">
                Choose one from the list, or create a new snippet.
              </p>
              <Button size="sm" onClick={() => openEditor(null)}>
                New snippet
              </Button>
            </div>
          )}
        </aside>
      </div>

      {pendingDelete ? (
        <ConfirmDialog
          title={`Delete ${pendingDelete.trigger}?`}
          description="This removes the snippet and its content. It cannot be undone."
          confirmLabel="Delete"
          danger
          onCancel={() => setPendingDelete(null)}
          onConfirm={() => {
            const target = pendingDelete;
            setPendingDelete(null);
            void removeSnippet(target.id).catch(reportError);
          }}
        />
      ) : null}
    </div>
  );
}

function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || target.isContentEditable;
}
