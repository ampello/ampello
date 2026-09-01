// SPDX-License-Identifier: GPL-3.0-or-later
import type { ReactNode } from "react";
import { cn } from "@/lib/cn";
import { Tooltip } from "./Tooltip";

export function SettingsSection({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <section className="mb-7">
      <h3 className="mb-2 px-1 text-[11px] font-medium uppercase tracking-[0.06em] text-muted">
        {title}
      </h3>
      <div className="divide-y divide-border overflow-hidden rounded-[10px] border border-border bg-surface">
        {children}
      </div>
    </section>
  );
}

export function SettingsRow({
  label,
  hint,
  htmlFor,
  control,
  className,
}: {
  label: string;
  hint?: string;
  htmlFor?: string;
  control: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("flex items-center justify-between gap-6 px-4 py-3", className)}>
      <div className="min-w-0 flex-1">
        <label htmlFor={htmlFor} className="text-[13px] text-primary">
          {hint ? <Tooltip content={hint}>{label}</Tooltip> : label}
        </label>
      </div>
      <div className="shrink-0">{control}</div>
    </div>
  );
}

export function SettingsBlock({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return <div className={cn("px-4 py-3.5", className)}>{children}</div>;
}
