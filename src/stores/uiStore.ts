// SPDX-License-Identifier: GPL-3.0-or-later
import { create } from "zustand";

export type View = "dashboard" | "snippets" | "settings" | "editor";

export type Scope =
  | { kind: "all" }
  | { kind: "favorites" }
  | { kind: "category"; id: string };

interface UiState {
  view: View;
  scope: Scope;
  sidebarCollapsed: boolean;
  search: string;

  editingId: string | null;

  draftCategoryId: string | null;

  setView: (view: View) => void;
  setScope: (scope: Scope) => void;
  toggleSidebar: () => void;
  setSearch: (search: string) => void;
  openEditor: (id: string | null) => void;
  closeEditor: () => void;
}

const SIDEBAR_KEY = "ampello.sidebarCollapsed";

export const useUiStore = create<UiState>((set) => ({
  view: "dashboard",
  scope: { kind: "all" },
  sidebarCollapsed: readBool(SIDEBAR_KEY, false),
  search: "",
  editingId: null,
  draftCategoryId: null,

  setView: (view) => set({ view }),
  setScope: (scope) => set({ scope, view: "snippets" }),
  toggleSidebar: () =>
    set((state) => {
      const next = !state.sidebarCollapsed;
      writeBool(SIDEBAR_KEY, next);
      return { sidebarCollapsed: next };
    }),
  setSearch: (search) => set({ search }),
  openEditor: (id) =>
    set((state) => ({
      view: "snippets",
      editingId: id ?? "new",
      draftCategoryId:
        id === null && state.view === "snippets" && state.scope.kind === "category"
          ? state.scope.id
          : null,
    })),
  closeEditor: () => set({ view: "snippets", editingId: null, draftCategoryId: null }),
}));

function readBool(key: string, fallback: boolean): boolean {
  try {
    const raw = localStorage.getItem(key);
    return raw === null ? fallback : raw === "true";
  } catch {
    return fallback;
  }
}

function writeBool(key: string, value: boolean) {
  try {
    localStorage.setItem(key, String(value));
  } catch {
  }
}
