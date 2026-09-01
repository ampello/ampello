// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect } from "react";
import { useSettingsStore } from "@/stores/settingsStore";
import type { ResolvedTheme } from "@/lib/types";

const DARK_QUERY = "(prefers-color-scheme: dark)";

function apply(theme: ResolvedTheme) {
  const root = document.documentElement;
  if (root.getAttribute("data-theme") !== theme) {
    root.setAttribute("data-theme", theme);
  }
  root.style.colorScheme = theme;
}

export function useThemeSync(): ResolvedTheme {
  const appearance = useSettingsStore((s) => s.settings.appearance);

  useEffect(() => {
    const media = window.matchMedia(DARK_QUERY);

    const resolve = (): ResolvedTheme => {
      if (appearance === "system") return media.matches ? "dark" : "light";
      return appearance;
    };

    apply(resolve());

    if (appearance !== "system") return;
    const onChange = () => apply(resolve());
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, [appearance]);

  if (appearance !== "system") return appearance;
  return typeof window !== "undefined" && window.matchMedia(DARK_QUERY).matches
    ? "dark"
    : "light";
}
