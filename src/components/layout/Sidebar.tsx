// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Folder, House, Layers, PanelLeft, Plus, Settings as SettingsIcon, Star } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { cn } from "@/lib/cn";
import { Button } from "@/components/ui/Button";
import { IconButton } from "@/components/ui/IconButton";
import { Menu } from "@/components/ui/Menu";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { useUiStore } from "@/stores/uiStore";
import type { Scope } from "@/stores/uiStore";
import { useDataStore } from "@/stores/dataStore";
import { reportError } from "@/stores/toastStore";
import type { Category } from "@/lib/types";

export function Sidebar() {
  const collapsed = useUiStore((s) => s.sidebarCollapsed);
  const toggle = useUiStore((s) => s.toggleSidebar);
  const view = useUiStore((s) => s.view);
  const scope = useUiStore((s) => s.scope);
  const setScope = useUiStore((s) => s.setScope);
  const setView = useUiStore((s) => s.setView);
  const openEditor = useUiStore((s) => s.openEditor);

  const categories = useDataStore((s) => s.categories);
  const snippets = useDataStore((s) => s.snippets);
  const addCategory = useDataStore((s) => s.addCategory);
  const renameCategory = useDataStore((s) => s.renameCategory);
  const removeCategory = useDataStore((s) => s.removeCategory);

  const [creating, setCreating] = useState(false);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [pendingDelete, setPendingDelete] = useState<Category | null>(null);

  const onSnippets = view === "snippets";
  const favoriteCount = snippets.filter((s) => s.favorite).length;

  return (
    <nav
      aria-label="Primary"
      className={cn(
        "flex h-full shrink-0 flex-col overflow-hidden bg-sidebar",

        "transition-[width] duration-[240ms] ease-[cubic-bezier(0.32,0.72,0,1)]",
        collapsed ? "w-[56px]" : "w-[228px]",
      )}
    >
      <div
        data-tauri-drag-region
        className={cn(
          "flex h-12 shrink-0 items-center border-b border-border",
          collapsed ? "justify-center px-0" : "pl-3",
        )}
      >
        <IconButton
          label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          size="sm"
          onClick={toggle}
          className="opacity-70 hover:opacity-100"
        >
          <PanelLeft size={15} strokeWidth={1.75} />
        </IconButton>
      </div>
      <div
        className={cn(
          "shrink-0 px-2.5 pt-3",
        )}
      >
        <NavItem
          icon={House}
          label="Home"
          collapsed={collapsed}
          active={view === "dashboard"}
          onClick={() => setView("dashboard")}
        />
      </div>
      <div
        className={cn(
          "flex shrink-0 px-2.5 pb-4 pt-2",
          collapsed && "justify-center",
        )}
      >
        <Button
          title="New snippet (Ctrl N)"
          aria-label={collapsed ? "New snippet" : undefined}
          onClick={() => openEditor(null)}
          className={cn(
            "overflow-hidden",

            collapsed
              ? "h-[30px] w-[36px] justify-center px-0"
              : "h-[30px] w-full justify-start gap-2.5 px-2",
          )}
        >
          <Plus size={15} strokeWidth={2} className="shrink-0 text-accent" />
          {collapsed ? null : <span className="truncate">New Snippet</span>}
        </Button>
      </div>

      <div
        className={cn(
          "flex-1 overflow-y-auto overflow-x-hidden px-2.5 pb-3",
        )}
      >
        <NavItem
          icon={Layers}
          label="All Snippets"
          count={snippets.length}
          collapsed={collapsed}
          active={onSnippets && scope.kind === "all"}
          onClick={() => setScope({ kind: "all" })}
        />
        <NavItem
          icon={Star}
          label="Favorites"
          count={favoriteCount}
          collapsed={collapsed}
          active={onSnippets && scope.kind === "favorites"}
          onClick={() => setScope({ kind: "favorites" })}
        />

        <div
          className={cn(
            "mb-1.5 mt-6 flex items-center justify-between overflow-hidden pl-2 pr-0.5",
            "transition-opacity duration-150",
            collapsed && "pointer-events-none opacity-0",
          )}
        >
          <p className="truncate text-[11px] font-medium uppercase tracking-[0.06em] text-muted">
            Collections
          </p>
          <IconButton
            label="New collection"
            size="sm"
            className="h-6 w-6 shrink-0"
            onClick={() => setCreating(true)}
          >
            <Plus size={13} strokeWidth={2} />
          </IconButton>
        </div>

        {categories.map((category) =>
          renamingId === category.id && !collapsed ? (
            <InlineNameInput
              key={category.id}
              initial={category.name}
              onCancel={() => setRenamingId(null)}
              onCommit={(name) => {
                setRenamingId(null);
                if (name !== category.name) {
                  void renameCategory(category.id, name).catch(reportError);
                }
              }}
            />
          ) : (
            <NavItem
              key={category.id}
              icon={Folder}
              label={category.name}
              count={snippets.filter((s) => s.categoryId === category.id).length}
              collapsed={collapsed}
              active={onSnippets && isCategory(scope, category.id)}
              onClick={() => setScope({ kind: "category", id: category.id })}
              menu={
                collapsed ? undefined : (
                  <Menu
                    label={`Actions for ${category.name}`}
                    className="h-6 w-6"
                    items={[
                      { label: "Rename", onSelect: () => setRenamingId(category.id) },
                      {
                        label: "Delete",
                        onSelect: () => setPendingDelete(category),
                        danger: true,
                        separatorBefore: true,
                      },
                    ]}
                  />
                )
              }
            />
          ),
        )}

        {creating && !collapsed ? (
          <InlineNameInput
            initial=""
            placeholder="Collection name"
            onCancel={() => setCreating(false)}
            onCommit={(name) => {
              setCreating(false);
              if (name) void addCategory(name).catch(reportError);
            }}
          />
        ) : null}
      </div>

      <div
        className={cn(
          "shrink-0 overflow-hidden border-t border-border px-2.5 py-2.5",
        )}
      >
        <NavItem
          icon={SettingsIcon}
          label="Settings"
          collapsed={collapsed}
          active={view === "settings"}
          onClick={() => setView("settings")}
        />
      </div>

      {pendingDelete ? (
        <ConfirmDialog
          title={`Delete “${pendingDelete.name}”?`}
          description="The collection is removed. Its snippets are kept and simply become uncategorised."
          confirmLabel="Delete"
          danger
          onCancel={() => setPendingDelete(null)}
          onConfirm={() => {
            const target = pendingDelete;
            setPendingDelete(null);
            if (isCategory(scope, target.id)) setScope({ kind: "all" });
            void removeCategory(target.id).catch(reportError);
          }}
        />
      ) : null}
    </nav>
  );
}

function isCategory(scope: Scope, id: string) {
  return scope.kind === "category" && scope.id === id;
}

interface NavItemProps {
  icon: LucideIcon;
  label: string;
  count?: number;
  collapsed: boolean;
  active: boolean;
  onClick: () => void;
  menu?: ReactNode;
}

function NavItem({
  icon: Icon,
  label,
  count,
  collapsed,
  active,
  onClick,
  menu,
}: NavItemProps) {
  return (
    <div className="group relative">
      <button
        type="button"
        onClick={onClick}
        title={collapsed ? label : undefined}
        aria-current={active ? "page" : undefined}
        className={cn(
          "flex h-[30px] w-full items-center overflow-hidden rounded-[8px]",
          "text-[13px] font-medium transition-colors duration-150",
          collapsed ? "justify-center px-0" : "gap-2.5 px-2",
          active
            ? "bg-accent-soft text-accent"
            : "text-secondary hover:bg-surface-2 hover:text-primary",
        )}
      >
        <Icon
          size={15}
          strokeWidth={1.75}
          className={cn(
            "shrink-0",
            active ? "text-accent" : "text-muted group-hover:text-secondary",
          )}
        />
        {collapsed ? null : (
          <span className="min-w-0 flex-1 truncate text-left">{label}</span>
        )}
        {!collapsed && typeof count === "number" && count > 0 ? (
          <span
            className={cn(
              "shrink-0 text-[11.5px] tabular-nums",
              active ? "text-accent" : "text-muted",
              Boolean(menu) && "group-hover:invisible",
            )}
          >
            {count}
          </span>
        ) : null}
      </button>
      {menu ? (
        <div className="absolute right-1 top-1/2 -translate-y-1/2 opacity-0 transition-opacity duration-150 group-hover:opacity-100 focus-within:opacity-100">
          {menu}
        </div>
      ) : null}
    </div>
  );
}

function InlineNameInput({
  initial,
  placeholder,
  onCommit,
  onCancel,
}: {
  initial: string;
  placeholder?: string;
  onCommit: (name: string) => void;
  onCancel: () => void;
}) {
  const ref = useRef<HTMLInputElement>(null);
  const [value, setValue] = useState(initial);

  useEffect(() => {
    ref.current?.focus();
    ref.current?.select();
  }, []);

  return (
    <input
      ref={ref}
      value={value}
      placeholder={placeholder}
      onChange={(event) => setValue(event.target.value)}
      onBlur={() => onCommit(value.trim())}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          event.preventDefault();
          onCommit(value.trim());
        } else if (event.key === "Escape") {
          event.preventDefault();
          onCancel();
        }
      }}
      className="h-[30px] w-full rounded-[8px] border border-accent bg-surface px-2 text-[13px] text-primary focus:outline-none"
    />
  );
}
