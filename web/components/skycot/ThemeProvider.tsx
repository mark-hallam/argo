"use client";

import { ThemeProvider as NextThemesProvider } from "next-themes";
import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { BUILTIN_THEMES, SKYCOT_THEME, getThemeById, resolveDataTheme, type ThemePair } from "@/lib/theme/themes";

interface TenantThemeContextValue {
  theme: ThemePair;
  setThemeById: (id: string) => void;
  availableThemes: ThemePair[];
}

const TenantThemeContext = createContext<TenantThemeContextValue | null>(null);

export function useTenantTheme(): TenantThemeContextValue {
  const ctx = useContext(TenantThemeContext);
  if (!ctx) throw new Error("useTenantTheme must be used inside <ThemeProvider>");
  return ctx;
}

interface ThemeProviderProps {
  /** Tenant id resolved server-side from the session/cookie. */
  initialThemeId?: string;
  /** Additional tenant-registered themes shipped at runtime. */
  registeredThemes?: ThemePair[];
  children: ReactNode;
}

export function ThemeProvider({ initialThemeId, registeredThemes = [], children }: ThemeProviderProps) {
  const allThemes = useMemo(
    () => [...BUILTIN_THEMES, ...registeredThemes.filter((t) => !BUILTIN_THEMES.some((b) => b.id === t.id))],
    [registeredThemes],
  );

  const [activeTheme, setActiveTheme] = useState<ThemePair>(
    () => allThemes.find((t) => t.id === initialThemeId) ?? SKYCOT_THEME,
  );

  function setThemeById(id: string) {
    const next = allThemes.find((t) => t.id === id) ?? getThemeById(id);
    setActiveTheme(next);
    try {
      window.localStorage.setItem("skycot.tenantTheme", next.id);
    } catch {
      /* localStorage unavailable; non-fatal */
    }
  }

  // Hydrate from localStorage on mount; preserves user choice across reloads.
  useEffect(() => {
    try {
      const stored = window.localStorage.getItem("skycot.tenantTheme");
      if (stored && stored !== activeTheme.id) {
        const found = allThemes.find((t) => t.id === stored);
        if (found) setActiveTheme(found);
      }
    } catch {
      /* ignore */
    }
    // intentionally empty deps — hydration runs once
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const ctxValue = useMemo<TenantThemeContextValue>(
    () => ({ theme: activeTheme, setThemeById, availableThemes: allThemes }),
    [activeTheme, allThemes],
  );

  return (
    <TenantThemeContext.Provider value={ctxValue}>
      <NextThemesProvider
        attribute="class"
        defaultTheme="system"
        enableSystem
        disableTransitionOnChange={false}
      >
        <DataThemeBridge theme={activeTheme}>{children}</DataThemeBridge>
      </NextThemesProvider>
    </TenantThemeContext.Provider>
  );
}

/**
 * next-themes manages the `light` / `dark` class on <html>. We
 * separately manage the `data-theme="<tenant>-<mode>"` attribute,
 * so the visible state combines both: a `.dark` class plus a
 * `data-theme` value. We listen for resolved-mode changes via a
 * MutationObserver on <html> to stay in sync without coupling
 * the two providers.
 */
function DataThemeBridge({ theme, children }: { theme: ThemePair; children: ReactNode }) {
  useEffect(() => {
    const html = document.documentElement;

    const apply = () => {
      const isDark = html.classList.contains("dark");
      html.setAttribute("data-theme", resolveDataTheme(theme, isDark ? "dark" : "light"));
    };

    apply();

    const observer = new MutationObserver(apply);
    observer.observe(html, { attributes: true, attributeFilter: ["class"] });
    return () => observer.disconnect();
  }, [theme]);

  return <>{children}</>;
}
