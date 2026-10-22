// SPDX-License-Identifier: GPL-3.0-or-later
import type { ReactNode } from "react";
import { Info } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { cn } from "@/lib/cn";
import { Tooltip } from "./Tooltip";

// Laid out the way PowerToys and Windows Settings are: a group heading, then
// one card per setting with a small gap between them.
export function SettingsSection({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <section className="mb-7">
      <h3 className="mb-2 text-[14px] font-semibold text-primary">{title}</h3>
      <div className="flex flex-col gap-1">{children}</div>
    </section>
  );
}

export const cardClass =
  "overflow-hidden rounded-[8px] border border-card-border bg-surface";

export function SettingsRow({
  label,
  description,
  hint,
  icon: Icon,
  htmlFor,
  control,
  className,
}: {
  label: string;
  // One line shown under the label.
  description?: string;
  // The longer explanation, behind the info button.
  hint?: string;
  icon?: LucideIcon;
  htmlFor?: string;
  control: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        cardClass,
        "flex min-h-[68px] items-center justify-between gap-6 px-4 py-3",
        className,
      )}
    >
      <div className="flex min-w-0 flex-1 items-center gap-4">
        {Icon ? (
          <Icon size={20} strokeWidth={1.5} className="shrink-0 text-primary" />
        ) : null}
        <div className="min-w-0">
          <div className="flex items-center gap-1.5">
            <label htmlFor={htmlFor} className="text-[14px] text-primary">
              {label}
            </label>
            {hint ? (
              <Tooltip content={hint}>
                <span
                  tabIndex={0}
                  aria-label={`About ${label}`}
                  className="inline-flex text-secondary hover:text-primary"
                >
                  <Info size={13} strokeWidth={1.75} />
                </span>
              </Tooltip>
            ) : null}
          </div>
          {description ? (
            <p className="text-[12px] leading-snug text-secondary">{description}</p>
          ) : null}
        </div>
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
  return <div className={cn(cardClass, "px-4 py-3.5", className)}>{children}</div>;
}
