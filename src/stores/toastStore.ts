// SPDX-License-Identifier: GPL-3.0-or-later
import { create } from "zustand";

export interface Toast {
  id: number;
  kind: "info" | "error";
  message: string;
}

interface ToastState {
  toasts: Toast[];
  push: (kind: Toast["kind"], message: string) => void;
  dismiss: (id: number) => void;
}

let nextId = 1;

export const useToastStore = create<ToastState>((set, get) => ({
  toasts: [],
  push(kind, message) {
    const id = nextId++;
    set({ toasts: [...get().toasts, { id, kind, message }] });
    setTimeout(() => get().dismiss(id), kind === "error" ? 6000 : 3000);
  },
  dismiss(id) {
    set({ toasts: get().toasts.filter((t) => t.id !== id) });
  },
}));

export function reportError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  useToastStore.getState().push("error", message);
}
