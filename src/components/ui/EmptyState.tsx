// SPDX-License-Identifier: GPL-3.0-or-later
import { cn } from "@/lib/cn";

interface EmptyStateProps {
  title?: string;
  description?: string;
}

export function EmptyState({ title, description }: EmptyStateProps) {
  return (
    <div className="motion-rise flex min-h-[280px] flex-col items-center justify-center px-6 py-14 text-center">
      {title ? <p className="text-[14px] font-medium text-primary">{title}</p> : null}
      {description ? (
        <p
          className={cn(
            "max-w-[380px] text-[13px] leading-relaxed text-secondary text-balance",
            title && "mt-1",
          )}
        >
          {description}
        </p>
      ) : null}
    </div>
  );
}
