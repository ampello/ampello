// SPDX-License-Identifier: GPL-3.0-or-later
import { invoke } from "@tauri-apps/api/core";
import type {
  Category,
  LibraryInfo,
  DatabaseInfo,
  Diagnostics,
  EngineStatus,
  ExportResult,
  ImportMode,
  ImportReport,
  NewSnippet,
  Settings,
  Snippet,
  SnippetPatch,
  SnippetSummary,
} from "./types";

// Every call into the Rust core goes through this module; nothing else imports
// `invoke` directly.
export class IpcError extends Error {
  readonly command: string;
  constructor(command: string, message: string) {
    super(message);
    this.name = "IpcError";
    this.command = command;
  }
}

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    throw new IpcError(
      command,
      "Ampello's core is not available. Run the desktop app with `npm run tauri dev` rather than opening the dev server in a browser.",
    );
  }
  try {
    return (await invoke(command, args)) as T;
  } catch (raw) {
    const message =
      typeof raw === "string"
        ? raw
        : raw instanceof Error
          ? raw.message
          : JSON.stringify(raw);
    throw new IpcError(command, message);
  }
}

export const getSettings = () => call<Settings>("get_settings");

export const updateSettings = (patch: Partial<Settings>) =>
  call<Settings>("update_settings", { patch });

export const listSnippets = () => call<SnippetSummary[]>("list_snippets");

export const searchSnippets = (query: string) =>
  call<SnippetSummary[]>("search_snippets", { query });

export const getSnippet = (id: string) => call<Snippet>("get_snippet", { id });

export const createSnippet = (snippet: NewSnippet) =>
  call<Snippet>("create_snippet", { snippet });

export const updateSnippet = (id: string, patch: SnippetPatch) =>
  call<Snippet>("update_snippet", { id, patch });

export const deleteSnippet = (id: string) => call<void>("delete_snippet", { id });

export const triggerAvailable = (trigger: string, exceptId?: string | null) =>
  call<boolean>("trigger_available", { trigger, exceptId: exceptId ?? null });

export const addAttachments = (snippetId: string, paths: string[]) =>
  call<Snippet>("add_attachments", { snippetId, paths });

export const pickAttachments = (snippetId: string) =>
  call<Snippet | null>("pick_attachments", { snippetId });

export const removeAttachment = (id: string) =>
  call<Snippet>("remove_attachment", { id });

export const reorderAttachments = (snippetId: string, ids: string[]) =>
  call<Snippet>("reorder_attachments", { snippetId, ids });

export const attachmentBytes = (id: string) =>
  call<ArrayBuffer | Uint8Array | number[]>("attachment_bytes", { id });

export const attachmentsSize = () => call<number>("attachments_size");

export const listCategories = () => call<Category[]>("list_categories");

export const createCategory = (name: string) =>
  call<Category>("create_category", { name });

export const renameCategory = (id: string, name: string) =>
  call<Category>("rename_category", { id, name });

export const deleteCategory = (id: string) => call<void>("delete_category", { id });

export const engineStatus = () => call<EngineStatus>("engine_status");

export const setExpansionEnabled = (enabled: boolean) =>
  call<Settings>("set_expansion_enabled", { enabled });

export const restartEngine = () => call<EngineStatus>("restart_engine");

export const shortcutError = () => call<string | null>("shortcut_error");

export const exportBackup = (format: "yaml" | "json") =>
  call<ExportResult | null>("export_backup", { format });

export const importBackup = (mode: ImportMode) =>
  call<ImportReport | null>("import_backup", { mode });

export const databaseInfo = () => call<DatabaseInfo>("database_info");

export const libraryInfo = () => call<LibraryInfo>("library_info");

/** Opens a folder picker. Resolves to null if the dialog was dismissed. */
export const chooseSharedLibrary = () =>
  call<string | null>("choose_shared_library");

export const usePersonalLibrary = () => call<void>("use_personal_library");

export const diagnostics = () => call<Diagnostics>("diagnostics");
