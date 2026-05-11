/**
 * Theme registry. The SPA ships with SKYCOT light/dark; tenants can
 * register additional themes via the gateway. The theme resolution
 * order is documented in
 * docs/build-plan/streams/adr-frontend-design-language.md §D3.
 *
 * A theme identifier is the value of `data-theme` on <html>. Pair
 * naming convention: `<name>-light` / `<name>-dark`.
 */
export type ThemeMode = "light" | "dark" | "system";

export interface ThemePair {
  /** Stable id used by the gateway to identify the tenant brand. */
  id: string;
  /** Human-friendly label shown in theme pickers. */
  label: string;
  /** data-theme value applied in light mode. */
  light: string;
  /** data-theme value applied in dark mode. */
  dark: string;
  /** Optional logo asset; SVG preferred. */
  logo?: { light?: string; dark?: string };
}

export const SKYCOT_THEME: ThemePair = {
  id: "skycot",
  label: "SKYCOT",
  light: "skycot-light",
  dark: "skycot-dark",
};

export const ACME_THEME: ThemePair = {
  id: "acme",
  label: "Acme Corp (demo white-label)",
  light: "acme-light",
  dark: "acme-dark",
};

export const BUILTIN_THEMES: ThemePair[] = [SKYCOT_THEME, ACME_THEME];

export function getThemeById(id: string | null | undefined): ThemePair {
  if (!id) return SKYCOT_THEME;
  return BUILTIN_THEMES.find((t) => t.id === id) ?? SKYCOT_THEME;
}

/**
 * Resolve which data-theme attribute value to apply, given the
 * tenant theme + the user's light/dark preference.
 */
export function resolveDataTheme(theme: ThemePair, resolvedMode: "light" | "dark"): string {
  return resolvedMode === "dark" ? theme.dark : theme.light;
}
