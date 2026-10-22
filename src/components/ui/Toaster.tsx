// SPDX-License-Identifier: GPL-3.0-or-later
import { X } from "lucide-react";
import { cn } from "@/lib/cn";
import { useToastStore } from "@/stores/toastStore";

export function Toaster() {
  const toasts = useToastStore((s) => s.toasts);
  const dismiss = useToastStore((s) => s.dismiss);

  if (toasts.length === 0) return null;

  return (
    <div
      role="status"
      aria-live="polite"
      className="pointer-events-none fixed bottom-4 right-4 z-50 flex w-[320px] flex-col gap-2"
    >
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={cn(
            "motion-toast pointer-events-auto flex items-start gap-2 rounded-[8px] border p-3",
            "shadow-[var(--shadow-md)]",
            toast.kind === "error"
              ? "border-danger/40 bg-danger-soft text-danger"
              : "border-border bg-surface text-primary",
          )}
        >
          <p className="min-w-0 flex-1 text-[12.5px] leading-relaxed">{toast.message}</p>
          <button
            type="button"
            aria-label="Dismiss"
            onClick={() => dismiss(toast.id)}
            className="-m-0.5 shrink-0 rounded-[4px] p-0.5 opacity-60 hover:opacity-100"
          >
            <X size={13} strokeWidth={2} />
          </button>
        </div>
      ))}
    </div>
  );
}
