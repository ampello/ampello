// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useRef } from "react";
import { Search, X } from "lucide-react";
import { cn } from "@/lib/cn";

let registered: HTMLInputElement | null = null;

export function focusSearch() {
  registered?.focus();
  registered?.select();
}

interface SearchFieldProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  className?: string;
}

export function SearchField({
  value,
  onChange,
  placeholder = "Search triggers and content…",
  className,
}: SearchFieldProps) {
  const ref = useRef<HTMLInputElement>(null);

  useEffect(() => {
    registered = ref.current;
    return () => {
      if (registered === ref.current) registered = null;
    };
  }, []);

  return (
    <div className={cn("relative w-full", className)}>
      <Search
        size={14}
        strokeWidth={1.75}
        className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-muted"
      />
      <input
        ref={ref}
        type="search"
        role="searchbox"
        aria-label="Search snippets"
        value={value}
        placeholder={placeholder}
        onChange={(event) => onChange(event.target.value)}
        className={cn(
          "h-8 w-full rounded-[8px] border border-border bg-surface pl-[30px] pr-8",
          "text-[13px] text-primary placeholder:text-muted",
          "transition-colors duration-150 focus:border-accent focus:outline-none",
          "[&::-webkit-search-cancel-button]:hidden",
        )}
      />
      {value ? (
        <button
          type="button"
          aria-label="Clear search"
          onClick={() => {
            onChange("");
            ref.current?.focus();
          }}
          className="absolute right-1.5 top-1/2 -translate-y-1/2 rounded-[6px] p-1 text-muted hover:bg-surface-2 hover:text-primary"
        >
          <X size={13} strokeWidth={2} />
        </button>
      ) : null}
    </div>
  );
}
