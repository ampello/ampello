// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { ROW_HEIGHT, SnippetRow } from "./SnippetRow";
import type { Category, SnippetSummary } from "@/lib/types";

interface SnippetListProps {
  snippets: SnippetSummary[];
  categories: Category[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onOpen: (id: string) => void;
  onToggleFavorite: (snippet: SnippetSummary) => void;
  onToggleEnabled: (snippet: SnippetSummary) => void;
  onDelete: (snippet: SnippetSummary) => void;
}

const OVERSCAN = 6;

export function SnippetList({
  snippets,
  categories,
  selectedId,
  onSelect,
  onOpen,
  onToggleFavorite,
  onToggleEnabled,
  onDelete,
}: SnippetListProps) {
  const viewportRef = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [height, setHeight] = useState(600);

  useLayoutEffect(() => {
    const element = viewportRef.current;
    if (!element) return;
    const observer = new ResizeObserver(([entry]) => {
      if (entry) setHeight(entry.contentRect.height);
    });
    observer.observe(element);
    setHeight(element.clientHeight);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    const element = viewportRef.current;
    if (!element || !selectedId) return;
    const index = snippets.findIndex((snippet) => snippet.id === selectedId);
    if (index < 0) return;
    const top = index * ROW_HEIGHT;
    const bottom = top + ROW_HEIGHT;
    if (top < element.scrollTop) {
      element.scrollTop = top;
    } else if (bottom > element.scrollTop + element.clientHeight) {
      element.scrollTop = bottom - element.clientHeight;
    }
  }, [selectedId, snippets]);

  const first = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
  const last = Math.min(
    snippets.length,
    Math.ceil((scrollTop + height) / ROW_HEIGHT) + OVERSCAN,
  );
  const visible = snippets.slice(first, last);

  const categoryName = (id: string | null) =>
    id ? (categories.find((category) => category.id === id)?.name ?? null) : null;

  return (
    <div
      ref={viewportRef}
      onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
      className="h-full overflow-y-auto px-3 pb-4"
    >
      <div
        role="listbox"
        aria-label="Snippets"
        style={{ height: snippets.length * ROW_HEIGHT }}
        className="relative"
      >
        {visible.map((snippet, index) => (
          <div
            key={snippet.id}
            role="presentation"
            style={{
              position: "absolute",
              top: (first + index) * ROW_HEIGHT,
              left: 0,
              right: 0,
              height: ROW_HEIGHT,
              padding: "3px 0",
            }}
          >
            <SnippetRow
              snippet={snippet}
              categoryName={categoryName(snippet.categoryId)}
              selected={snippet.id === selectedId}
              onSelect={() => onSelect(snippet.id)}
              onOpen={() => onOpen(snippet.id)}
              onToggleFavorite={() => onToggleFavorite(snippet)}
              onToggleEnabled={() => onToggleEnabled(snippet)}
              onDelete={() => onDelete(snippet)}
            />
          </div>
        ))}
      </div>
    </div>
  );
}
