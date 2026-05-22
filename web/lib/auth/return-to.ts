/**
 * Session-expired returnTo handling.
 *
 * When the gateway 401s on an expired session JWT, the SPA captures
 * the current pathname + search as `returnTo` and threads it through
 * `/auth/signin?returnTo=<encoded>` → magic-link mint → verify page
 * → post-verify redirect. The `safeReturnToOrHome` gate is the
 * load-bearing security boundary: an attacker who can craft a
 * malicious `returnTo` (e.g. via a phishing-style email link to
 * `/auth/signin?returnTo=https://evil.com/`) would otherwise be able
 * to bounce the user off-site after a successful re-auth.
 *
 * The whitelist is the set of in-product route prefixes the SPA
 * actually owns. Anything outside (including marketing surfaces,
 * external URLs, and auth surfaces themselves) folds to "/".
 */
export const INTERNAL_PATH_PREFIXES: readonly string[] = [
  "/app/",
  "/workflows/",
  "/audit/",
  "/cohorts/",
  "/approvals/",
  "/skills/",
  "/settings/",
  "/onboarding/",
];

const SCHEME_RE = /^[a-z][a-z0-9+\-.]*:/i;

/**
 * Returns the `returnTo` value iff it points at a whitelisted
 * in-product path; otherwise returns `'/'`. Rejects:
 *   - missing / empty / non-string values
 *   - absolute URLs (`https://...`, `http://...`, custom schemes)
 *   - `javascript:` / `data:` / other URI schemes
 *   - protocol-relative URLs (`//evil.com/...`)
 *   - paths that don't start with `/`
 *   - paths whose pathname segment is outside [INTERNAL_PATH_PREFIXES]
 */
export function safeReturnToOrHome(returnTo: string | null | undefined): string {
  if (typeof returnTo !== "string" || returnTo.length === 0) return "/";
  if (returnTo.startsWith("//")) return "/";
  if (SCHEME_RE.test(returnTo)) return "/";
  if (!returnTo.startsWith("/")) return "/";

  const pathOnly = returnTo.split("?", 1)[0]!.split("#", 1)[0]!;
  const matchesWhitelist = INTERNAL_PATH_PREFIXES.some((prefix) => {
    if (pathOnly.startsWith(prefix)) return true;
    return pathOnly === prefix.slice(0, -1);
  });
  return matchesWhitelist ? returnTo : "/";
}

/**
 * Read `returnTo` from a URLSearchParams-like object. Returns `null`
 * when the key is absent or the params object is null.
 */
export function parseReturnTo(
  searchParams: URLSearchParams | { get(name: string): string | null } | null | undefined,
): string | null {
  if (!searchParams) return null;
  return searchParams.get("returnTo");
}

/**
 * Snapshot `window.location.pathname + search` for use as a
 * `returnTo` value. Returns `'/'` when called outside the browser
 * (SSR/RSC), or when the snapshot would be an auth surface (which
 * would create a redirect loop after re-auth).
 */
export function captureCurrentLocationAsReturnTo(): string {
  if (typeof window === "undefined") return "/";
  const path = window.location.pathname || "/";
  if (path.startsWith("/auth/")) return "/";
  return path + (window.location.search || "");
}

/**
 * Build the `/auth/signin` URL with `returnTo` attached, skipping
 * the parameter when the current path is home or already inside the
 * auth surface (no point round-tripping the user back to signin).
 */
export function buildSigninUrlWithReturnTo(currentPath: string): string {
  if (!currentPath || currentPath === "/" || currentPath.startsWith("/auth/")) {
    return "/auth/signin";
  }
  return `/auth/signin?returnTo=${encodeURIComponent(currentPath)}`;
}

/**
 * Session-storage key used by the verify page to hand off the
 * "show welcome-back banner" signal to the destination route. The
 * destination's mount of [WelcomeBackBanner] consumes (and clears)
 * the flag on first render.
 */
export const WELCOME_BACK_FLAG_KEY = "skycot:welcome-back";

/**
 * Local-storage key used to carry a pending `returnTo` across the
 * magic-link round-trip. The signin page stores the value when the
 * user lands with `?returnTo=`; the verify page consumes it after
 * a successful `POST /api/auth/verify`. localStorage (not session-
 * storage) is required because most email clients open the magic
 * link in a fresh tab, which gets its own sessionStorage but
 * inherits the origin's localStorage.
 *
 * The value is wrapped with a timestamp and given a 15-minute TTL
 * (matching the magic-link expiry); stale entries are dropped on
 * read.
 */
const RETURN_TO_PENDING_KEY = "skycot:return-to-pending";
const RETURN_TO_PENDING_TTL_MS = 15 * 60 * 1000;

interface ReturnToPending {
  p: unknown;
  t: unknown;
}

/**
 * Persist `returnTo` across the magic-link round-trip. No-ops under
 * SSR, when localStorage is unavailable, and when the value would
 * be folded to `'/'` by [safeReturnToOrHome] (nothing useful to
 * store).
 */
export function storeReturnToPending(returnTo: string | null | undefined): void {
  if (typeof window === "undefined") return;
  const safe = safeReturnToOrHome(returnTo);
  if (safe === "/") return;
  try {
    window.localStorage.setItem(
      RETURN_TO_PENDING_KEY,
      JSON.stringify({ p: safe, t: Date.now() }),
    );
  } catch {
    /* storage disabled (private window, quota) — degrade silently */
  }
}

/**
 * Read + clear the pending `returnTo`. Re-validates via
 * [safeReturnToOrHome] on read so a tampered localStorage value
 * still folds to `'/'`. Drops entries older than 15 minutes.
 */
export function consumeReturnToPending(): string | null {
  if (typeof window === "undefined") return null;
  let raw: string | null = null;
  try {
    raw = window.localStorage.getItem(RETURN_TO_PENDING_KEY);
    if (raw !== null) window.localStorage.removeItem(RETURN_TO_PENDING_KEY);
  } catch {
    return null;
  }
  if (!raw) return null;
  let parsed: ReturnToPending;
  try {
    parsed = JSON.parse(raw) as ReturnToPending;
  } catch {
    return null;
  }
  if (typeof parsed.p !== "string") return null;
  if (typeof parsed.t !== "number") return null;
  if (Date.now() - parsed.t > RETURN_TO_PENDING_TTL_MS) return null;
  const safe = safeReturnToOrHome(parsed.p);
  return safe === "/" ? null : safe;
}
