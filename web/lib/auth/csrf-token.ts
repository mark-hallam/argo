/**
 * CSRF double-submit token helper
 * (Stream `claude-rebase-pr-1835-cookies`).
 *
 * The gateway mints two browser cookies on a successful magic-link
 * verify: `__Host-session` (HTTPOnly, the JWT) and `__Host-csrf`
 * (NOT HTTPOnly, the CSRF token = the JWT's `jti` claim). The SPA
 * reads the csrf value either out of `document.cookie` directly or
 * via the same-origin XHR endpoint `GET /api/v1/auth/csrf`, then
 * echoes it back in the `X-CSRF-Token` header on every mutating
 * request to the gateway.
 *
 * The double-submit pattern only works when:
 * - The token cookie is not HttpOnly (so JS can read it), AND
 * - The header is custom (browsers refuse to set custom headers on
 *   simple cross-site form POSTs).
 *
 * Both invariants are enforced server-side by `middleware_csrf` on
 * the gateway. This module is the client-side participation.
 */

const CSRF_COOKIE = "__Host-csrf";
const FETCH_FALLBACK_URL = "/api/v1/auth/csrf";

/**
 * Return the current CSRF token, or `undefined` if no cookie is set.
 *
 * SSR: returns `undefined` (no `document` in the Node environment).
 * Callers in server components should fetch the cookie value via
 * Next.js `cookies()` directly.
 */
export function readCsrfTokenFromCookie(): string | undefined {
  if (typeof document === "undefined") return undefined;
  const target = `${CSRF_COOKIE}=`;
  for (const pair of document.cookie.split(";")) {
    const trimmed = pair.trim();
    if (trimmed.startsWith(target)) {
      return decodeURIComponent(trimmed.slice(target.length));
    }
  }
  return undefined;
}

/**
 * Fetch the CSRF token via the gateway's XHR endpoint. Used as a
 * fallback when `document.cookie` is unreadable (e.g. some browser
 * privacy modes block cookie reads from script) or when the SPA
 * boots before the cookie is materialised.
 *
 * Resolves to `undefined` on any error / non-2xx — caller should
 * fall back to omitting the header (the gateway will then reject
 * the mutating request with a clean 403).
 */
export async function fetchCsrfTokenViaXhr(): Promise<string | undefined> {
  try {
    const res = await fetch(FETCH_FALLBACK_URL, {
      method: "GET",
      credentials: "same-origin",
      cache: "no-store",
    });
    if (!res.ok) return undefined;
    const body = (await res.json()) as { token?: string };
    return typeof body.token === "string" ? body.token : undefined;
  } catch {
    return undefined;
  }
}

/**
 * Resolve a CSRF token for outgoing mutating requests. Prefers the
 * cookie (zero-cost) and falls back to the XHR endpoint exactly
 * once if the cookie is missing.
 */
export async function resolveCsrfToken(): Promise<string | undefined> {
  const cached = readCsrfTokenFromCookie();
  if (cached) return cached;
  return fetchCsrfTokenViaXhr();
}
