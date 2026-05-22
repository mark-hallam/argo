"use client";

import { Moon, Sun } from "lucide-react";
import { useTheme } from "next-themes";
import { useEffect, useState } from "react";

/**
 * Sidebar light/dark toggle. Placed next to the UserPill so it sits in
 * a discoverable, persistent spot (operator memory:
 * project_multi_tenant_theming.md). Persists user choice via
 * next-themes' default localStorage mechanism. We render a stable
 * placeholder before mount to avoid a hydration mismatch — `resolvedTheme`
 * is undefined during SSR.
 */
export function ThemeToggle() {
  const { resolvedTheme, setTheme } = useTheme();
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);

  const isDark = mounted && resolvedTheme === "dark";
  const nextLabel = isDark ? "Switch to light theme" : "Switch to dark theme";

  return (
    <button
      type="button"
      data-testid="appshell-theme-toggle"
      aria-label={nextLabel}
      title={nextLabel}
      onClick={() => setTheme(isDark ? "light" : "dark")}
      className="mt-1 inline-flex h-9 w-9 items-center justify-center self-end rounded-[var(--radius-md)] text-[var(--muted-foreground)] hover:bg-[var(--accent)] hover:text-[var(--foreground)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] transition-colors duration-[var(--duration-fast)]"
    >
      {mounted ? (
        isDark ? <Sun className="size-4" /> : <Moon className="size-4" />
      ) : (
        <span aria-hidden className="size-4" />
      )}
    </button>
  );
}
