"use client";

import { Moon, Sun } from "lucide-react";
import { useTheme } from "next-themes";
import { useEffect, useState } from "react";
import { writeColorModeCookie } from "@/lib/theme/cookie";

/**
 * Sidebar light/dark toggle. Placed next to the UserPill so it sits in
 * a discoverable, persistent spot (operator memory:
 * project_multi_tenant_theming.md). Persists user choice via the
 * provider's explicit `theme` localStorage key. We render a stable
 * placeholder before mount to avoid a hydration mismatch — `resolvedTheme`
 * is undefined during SSR.
 *
 * Also mirrors the resolved theme into the `skycot-color-mode` cookie
 * (whole-site F07 fix) so the next SSR pass on a marketing route lands
 * on the user's preference instead of always-light.
 */
interface ThemeToggleProps {
  /**
   * Override the default sidebar styling. Marketing nav supplies a
   * register-appropriate class set so the toggle blends with the
   * `mk-top` strip instead of using the in-app sidebar tokens.
   */
  className?: string;
}

const DEFAULT_CLASSES =
  "mt-1 inline-flex h-9 w-9 items-center justify-center self-end rounded-[var(--radius-md)] text-[var(--muted-foreground)] hover:bg-[var(--accent)] hover:text-[var(--foreground)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] transition-colors duration-[var(--duration-fast)]";

export function ThemeToggle({ className }: ThemeToggleProps = {}) {
  const { resolvedTheme, setTheme } = useTheme();
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);

  // Mirror next-themes' resolved value into the SSR-readable cookie.
  // Covers all theme-change sources (this toggle, ThemePicker, system
  // pref flips with enableSystem) — not just the click handler below.
  useEffect(() => {
    if (!mounted) return;
    if (resolvedTheme === "dark" || resolvedTheme === "light") {
      writeColorModeCookie(resolvedTheme);
    }
  }, [mounted, resolvedTheme]);

  const isDark = mounted && resolvedTheme === "dark";
  const nextLabel = isDark ? "Switch to light theme" : "Switch to dark theme";

  function handleToggle() {
    const htmlIsDark = document.documentElement.classList.contains("dark");
    const next = htmlIsDark ? "light" : "dark";
    setTheme(next);
    writeColorModeCookie(next);
  }

  return (
    <button
      type="button"
      data-testid="appshell-theme-toggle"
      aria-label={nextLabel}
      title={nextLabel}
      onClick={handleToggle}
      className={className ?? DEFAULT_CLASSES}
    >
      {mounted ? (
        isDark ? <Sun className="size-4" /> : <Moon className="size-4" />
      ) : (
        <span aria-hidden className="size-4" />
      )}
    </button>
  );
}
