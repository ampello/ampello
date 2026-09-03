// SPDX-License-Identifier: GPL-3.0-or-later
import { create } from "zustand";

export type View = "dashboard" | "snippets" | "settings";

export type Scope =
  | { kind: "all" }
  | { kind: "favorites" }
  | { kind: "category"; id: string };

interface UiState {
  view: View;
  scope: Scope;
  sidebarCollapsed: boolean;
  search: string;

  paneWidth: number;
  paneExpanded: boolean;

  editingId: string | null;

  draftCategoryId: string | null;

  setView: (view: View) => void;
  setScope: (scope: Scope) => void;
  toggleSidebar: () => void;
  setPaneWidth: (width: number) => void;
  setPaneExpanded: (expanded: boolean) => void;
  setSearch: (search: string) => void;
  openEditor: (id: string | null) => void;
  closeEditor: () => void;
}

const SIDEBAR_KEY = "ampello.sidebarCollapsed";
const PANE_WIDTH_KEY = "ampello.paneWidth";
const PANE_EXPANDED_KEY = "ampello.paneExpanded";

export const PANE_DEFAULT_WIDTH = 420;
export const PANE_MIN_WIDTH = 340;
export const LIST_MIN_WIDTH = 280;

export const useUiStore = create<UiState>((set) => ({
  view: "dashboard",
  scope: { kind: "all" },
  sidebarCollapsed: readBool(SIDEBAR_KEY, false),
  search: "",
  paneWidth: readNumber(PANE_WIDTH_KEY, PANE_DEFAULT_WIDTH),
  paneExpanded: readBool(PANE_EXPANDED_KEY, false),
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
  setPaneWidth: (width) =>
    set(() => {
      writeNumber(PANE_WIDTH_KEY, width);
      return { paneWidth: width };
    }),
  setPaneExpanded: (expanded) =>
    set(() => {
      writeBool(PANE_EXPANDED_KEY, expanded);
      return { paneExpanded: expanded };
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

function readNumber(key: string, fallback: number): number {
  try {
    const raw = localStorage.getItem(key);
    if (raw === null) return fallback;
    const value = Number(raw);
    return Number.isFinite(value) ? value : fallback;
  } catch {
    return fallback;
  }
}

function writeNumber(key: string, value: number) {
  try {
    localStorage.setItem(key, String(value));
  } catch {
  }
}

function writeBool(key: string, value: boolean) {
  try {
    localStorage.setItem(key, String(value));
  } catch {
  }
}
