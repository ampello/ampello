// SPDX-License-Identifier: GPL-3.0-or-later
import { TriangleAlert } from "lucide-react";

export function BootErrorView({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <div className="relative flex h-full w-full items-center justify-center bg-bg px-8">
      <div
        data-tauri-drag-region
        className="absolute inset-x-0 top-0 h-12"
        aria-hidden="true"
      />
      <div className="max-w-[440px] text-center">
        <div className="mx-auto mb-4 flex h-10 w-10 items-center justify-center rounded-[12px] border border-border bg-danger-soft text-danger">
          <TriangleAlert size={18} strokeWidth={1.75} />
        </div>
        <p className="text-[14px] font-medium text-primary">Ampello could not start</p>
        <p className="mt-1.5 text-[13px] leading-relaxed text-secondary">{message}</p>
        <button
          type="button"
          onClick={onRetry}
          className="mt-5 inline-flex h-8 items-center rounded-[8px] bg-accent px-3 text-[13px] font-medium text-accent-contrast hover:bg-accent-hover"
        >
          Try again
        </button>
      </div>
    </div>
  );
}
