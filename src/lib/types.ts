// SPDX-License-Identifier: GPL-3.0-or-later
export interface Snippet {
  id: string;
  trigger: string;
  content: string;
  enabled: boolean;
  favorite: boolean;
  categoryId: string | null;
  usageCount: number;

  lastUsedAt: number | null;
  createdAt: number;
  updatedAt: number;

  attachments: Attachment[];

  attachmentsFirst: boolean;

  strictOrder: boolean;
}

export interface Attachment {
  id: string;
  snippetId: string;
  position: number;
  name: string;

  mime: string;
  digest: string;
  sizeBytes: number;
  createdAt: number;

  present: boolean;
}

export interface SnippetSummary {
  id: string;
  trigger: string;
  preview: string;
  contentLength: number;
  enabled: boolean;
  favorite: boolean;
  categoryId: string | null;
  usageCount: number;
  lastUsedAt: number | null;
  createdAt: number;
  updatedAt: number;

  attachmentCount: number;
}

export interface Category {
  id: string;
  name: string;
  position: number;
  createdAt: number;
}

export interface NewSnippet {
  trigger: string;
  content: string;
  categoryId?: string | null;
}

export interface SnippetPatch {
  trigger?: string;
  content?: string;
  enabled?: boolean;
  favorite?: boolean;
  categoryId?: string | null;
  attachmentsFirst?: boolean;
  strictOrder?: boolean;
}

export type Appearance = "light" | "dark" | "system";
export type ResolvedTheme = "light" | "dark";

export type BoundaryMode = "word" | "anywhere";

export type InjectionMode = "auto" | "paste" | "type";

export type TypingSpeed = "fast" | "balanced" | "careful";

export type ClipboardMode = "paste" | "type";

export interface Settings {
  appearance: Appearance;
  expansionEnabled: boolean;
  launchAtStartup: boolean;
  globalShortcut: string;
  boundaryMode: BoundaryMode;

  preserveBoundaryChar: boolean;

  restoreClipboard: boolean;
  injectionMode: InjectionMode;
  typingSpeed: TypingSpeed;

  clipboardShortcutEnabled: boolean;

  clipboardShortcut: string;
  clipboardMode: ClipboardMode;
  closeToTray: boolean;

  attachmentSettleMs: number;
}

export const MIN_SETTLE_MS = 100;
export const MAX_SETTLE_MS = 5000;

export type ImportMode = "skip" | "replace";

export interface ImportReport {
  added: number;
  replaced: number;
  skipped: number;
  collectionsCreated: number;

  problems: string[];
}

export interface ExportResult {
  path: string;
  snippets: number;
  collections: number;
}

export interface EngineStatus {
  running: boolean;

  enabled: boolean;
  triggerCount: number;
  error: string | null;
  platform: string;

  keystrokesSeen: number;
  expansions: number;

  lastExpansionError: string | null;
}

export interface Diagnostics {
  logDirectory: string | null;
}

export interface LibraryInfo {
  path: string;
  shared: boolean;
  personalPath: string;
  defaultSharedPath: string;
  problem: string | null;
}

export interface DatabaseInfo {
  path: string;

  recoveredFrom: string | null;
  snippetCount: number;
  categoryCount: number;
  schemaVersion: number;
  sizeBytes: number;
}
