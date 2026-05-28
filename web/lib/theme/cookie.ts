/**
 * Color-mode cookie — bridges in-app theme preference to marketing
 * pages so SSR can render the right `class="dark"` (or no class for
 * light) on <html> without a client-side flash.
 *
 * Shadows next-themes' localStorage value (key "theme"). ThemeToggle
 * writes both: localStorage drives next-themes hydration, the cookie
 * drives the next SSR render.
 *
 * Read side runs server-only via a dynamic `next/headers` import
 * (matches the P0 #2027 pattern in web/lib/auth/session-cookie.ts),
 * so this module remains safe to import from Client Components.
 */

export const THEME_COOKIE_NAME = "skycot-color-mode";
export type ColorMode = "light" | "dark";

function parseColorMode(raw: string | undefined): ColorMode | null {
  if (raw === "light" || raw === "dark") return raw;
  return null;
}

/**
 * Server-side read of the color-mode cookie. Returns `null` on the
 * client or when the cookie is absent / malformed. Use this in
 * server components (e.g. app/layout.tsx) to set the initial
 * `<html class>` so the first paint matches the user's preference.
 */
export async function readColorModeCookie(): Promise<ColorMode | null> {
  if (typeof window !== "undefined") return null;
  try {
    const { cookies } = await import("next/headers");
    const store = await cookies();
    return parseColorMode(store.get(THEME_COOKIE_NAME)?.value);
  } catch {
    return null;
  }
}

/**
 * Client-side write. Call alongside next-themes' setTheme(...) so the
 * cookie tracks the user's actual resolved preference for the next
 * SSR render. Path=/ covers /landing, /pricing, /app, /onboarding etc.
 */
export function writeColorModeCookie(mode: ColorMode): void {
  if (typeof document === "undefined") return;
  const oneYearSeconds = 60 * 60 * 24 * 365;
  document.cookie =
    `${THEME_COOKIE_NAME}=${mode}; Path=/; Max-Age=${oneYearSeconds}; SameSite=Lax`;
}
