// SPDX-License-Identifier: GPL-3.0-or-later
import { Paperclip, Star } from "lucide-react";
import { cn } from "@/lib/cn";
import { Menu } from "@/components/ui/Menu";
import type { SnippetSummary } from "@/lib/types";

export const ROW_HEIGHT = 68;

interface SnippetRowProps {
  snippet: SnippetSummary;
  categoryName: string | null;
  selected: boolean;
  onSelect: () => void;
  onOpen: () => void;
  onToggleFavorite: () => void;
  onToggleEnabled: () => void;
  onDelete: () => void;
}

export function SnippetRow({
  snippet,
  categoryName,
  selected,
  onSelect,
  onOpen,
  onToggleFavorite,
  onToggleEnabled,
  onDelete,
}: SnippetRowProps) {
  return (
    <div
      role="option"
      aria-selected={selected}
      tabIndex={-1}
      onClick={onSelect}
      onDoubleClick={onOpen}
      className={cn(
        "group flex h-full w-full cursor-default items-center gap-3 rounded-[9px] px-3",
        "transition-colors duration-75",
        selected ? "bg-accent-soft" : "hover:bg-surface-2",
      )}
    >
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 items-baseline gap-2.5">
          <span
            className={cn(
              "shrink-0 font-mono text-[13px] font-medium",
              snippet.enabled ? "text-accent" : "text-muted line-through",
            )}
          >
            {snippet.trigger}
          </span>
          {!snippet.enabled ? (
            <span className="shrink-0 rounded-[6px] border border-border px-1 text-[10.5px] uppercase tracking-[0.04em] text-muted">
              Off
            </span>
          ) : null}
          {snippet.attachmentCount > 0 ? (
            <span
              className="flex shrink-0 items-center gap-0.5 text-[11px] text-muted"
              title={`${snippet.attachmentCount} attached file${
                snippet.attachmentCount === 1 ? "" : "s"
              }`}
            >
              <Paperclip size={11} strokeWidth={1.75} />
              {snippet.attachmentCount}
            </span>
          ) : null}
        </div>
        <p className="mt-1 truncate text-[12.5px] text-secondary">
          {snippet.preview || <span className="italic text-muted">Empty</span>}
        </p>
      </div>

      {categoryName ? (
        <span className="hidden shrink-0 rounded-[6px] bg-surface-3 px-1.5 py-0.5 text-[11px] text-secondary sm:inline">
          {categoryName}
        </span>
      ) : null}

      <button
        type="button"
        aria-label={snippet.favorite ? "Remove from favorites" : "Add to favorites"}
        aria-pressed={snippet.favorite}
        onClick={(event) => {
          event.stopPropagation();
          onToggleFavorite();
        }}
        className={cn(
          "shrink-0 rounded-[5px] p-1.5 transition-colors duration-150",
          snippet.favorite
            ? "text-accent"
            : "text-muted opacity-0 hover:text-primary group-hover:opacity-100 focus-visible:opacity-100",
        )}
      >
        <Star
          size={15}
          strokeWidth={1.75}
          fill={snippet.favorite ? "currentColor" : "none"}
        />
      </button>

      <div className="shrink-0 opacity-0 transition-opacity duration-150 group-hover:opacity-100 focus-within:opacity-100">
        <Menu
          label={`Actions for ${snippet.trigger}`}
          items={[
            { label: "Edit", onSelect: onOpen },
            {
              label: snippet.enabled ? "Disable" : "Enable",
              onSelect: onToggleEnabled,
            },
            {
              label: "Delete",
              onSelect: onDelete,
              danger: true,
              separatorBefore: true,
            },
          ]}
        />
      </div>
    </div>
  );
}
