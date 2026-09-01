// SPDX-License-Identifier: GPL-3.0-or-later
import { create } from "zustand";
import * as ipc from "@/lib/ipc";
import type { Category, NewSnippet, Snippet, SnippetPatch, SnippetSummary } from "@/lib/types";

interface DataState {
  snippets: SnippetSummary[];
  categories: Category[];
  status: "idle" | "loading" | "ready" | "error";
  error: string | null;

  query: string;
  results: SnippetSummary[] | null;
  searching: boolean;

  load: () => Promise<void>;
  refresh: () => Promise<void>;
  setQuery: (query: string) => void;

  createSnippet: (input: NewSnippet) => Promise<Snippet>;
  saveSnippet: (id: string, patch: SnippetPatch) => Promise<Snippet>;
  removeSnippet: (id: string) => Promise<void>;
  setFavorite: (id: string, favorite: boolean) => Promise<void>;
  setEnabled: (id: string, enabled: boolean) => Promise<void>;

  addCategory: (name: string) => Promise<Category>;
  renameCategory: (id: string, name: string) => Promise<void>;
  removeCategory: (id: string) => Promise<void>;
}

const SEARCH_DEBOUNCE_MS = 80;
let searchTimer: ReturnType<typeof setTimeout> | null = null;
let searchSequence = 0;

export const useDataStore = create<DataState>((set, get) => ({
  snippets: [],
  categories: [],
  status: "idle",
  error: null,
  query: "",
  results: null,
  searching: false,

  async load() {
    set({ status: "loading", error: null });
    try {
      const [snippets, categories] = await Promise.all([
        ipc.listSnippets(),
        ipc.listCategories(),
      ]);
      set({ snippets, categories, status: "ready" });
    } catch (error) {
      set({ status: "error", error: message(error) });
    }
  },

  async refresh() {
    const [snippets, categories] = await Promise.all([
      ipc.listSnippets(),
      ipc.listCategories(),
    ]);
    set({ snippets, categories });
    const { query } = get();
    if (query.trim()) {
      set({ results: await ipc.searchSnippets(query) });
    }
  },

  setQuery(query) {
    set({ query });
    if (searchTimer) clearTimeout(searchTimer);

    if (!query.trim()) {
      searchSequence++;
      set({ results: null, searching: false });
      return;
    }

    set({ searching: true });
    searchTimer = setTimeout(() => {
      const sequence = ++searchSequence;
      ipc
        .searchSnippets(query)
        .then((results) => {
          if (sequence === searchSequence) set({ results, searching: false });
        })
        .catch((error) => {
          if (sequence === searchSequence) {
            set({ searching: false, error: message(error) });
          }
        });
    }, SEARCH_DEBOUNCE_MS);
  },

  async createSnippet(input) {
    const created = await ipc.createSnippet(input);
    await get().refresh();
    return created;
  },

  async saveSnippet(id, patch) {
    const saved = await ipc.updateSnippet(id, patch);
    await get().refresh();
    return saved;
  },

  async removeSnippet(id) {
    await ipc.deleteSnippet(id);
    await get().refresh();
  },

  async setFavorite(id, favorite) {
    patchLocally(set, get, id, { favorite });
    try {
      await ipc.updateSnippet(id, { favorite });
    } catch (error) {
      patchLocally(set, get, id, { favorite: !favorite });
      throw error;
    }
  },

  async setEnabled(id, enabled) {
    patchLocally(set, get, id, { enabled });
    try {
      await ipc.updateSnippet(id, { enabled });
    } catch (error) {
      patchLocally(set, get, id, { enabled: !enabled });
      throw error;
    }
  },

  async addCategory(name) {
    const created = await ipc.createCategory(name);
    set({ categories: await ipc.listCategories() });
    return created;
  },

  async renameCategory(id, name) {
    await ipc.renameCategory(id, name);
    set({ categories: await ipc.listCategories() });
  },

  async removeCategory(id) {
    await ipc.deleteCategory(id);
    await get().refresh();
  },
}));

function patchLocally(
  set: (partial: Partial<DataState>) => void,
  get: () => DataState,
  id: string,
  patch: Partial<SnippetSummary>,
) {
  const apply = (rows: SnippetSummary[]) =>
    rows.map((row) => (row.id === id ? { ...row, ...patch } : row));
  const { snippets, results } = get();
  set({ snippets: apply(snippets), results: results ? apply(results) : null });
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
