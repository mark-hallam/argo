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

/**
 * The slot a tenant's persisted white-label accent occupies. Its
 * `data-theme` ids match `TENANT_THEME_ID` in `tenant-theme.ts`, so the
 * CSS synthesized from `GET /api/v1/tenant/theme` (and injected by
 * `TenantThemeStyle`) applies when this theme is active. Registering it
 * here is what lets `ThemeProvider.setThemeById("tenant")` resolve the
 * pair (otherwise it falls back to SKYCOT). It is only ever activated
 * when the gateway reports `is_override: true`, so the locked SKYCOT
 * default is unaffected for non-white-label tenants.
 */
export const TENANT_OVERRIDE_THEME: ThemePair = {
  id: "tenant",
  label: "Workspace brand",
  light: "tenant-light",
  dark: "tenant-dark",
};

export const BUILTIN_THEMES: ThemePair[] = [SKYCOT_THEME, ACME_THEME, TENANT_OVERRIDE_THEME];

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
