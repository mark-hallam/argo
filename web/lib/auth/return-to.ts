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
 * to bounce the user off-site after a successful re-auth ("open
 * redirect").
 *
 * The actual security requirement is *structural*, not a list of
 * route prefixes: a `returnTo` is safe iff it is a same-origin
 * RELATIVE path — it stays inside this app's own origin and cannot
 * be coerced into navigating off-site or executing a scheme. We
 * enforce exactly that and nothing more. The previous implementation
 * gated on a hand-maintained `INTERNAL_PATH_PREFIXES` allowlist; that
 * list drifted behind the real routes (e.g. `/team`, `/pitch`,
 * `/coach`, `/genesis`, `/workers`, `/billing`, …), so legitimate
 * in-app deep-links silently folded to `/` after re-auth. Encoding
 * the security property directly removes that whole class of drift.
 */

// Matches a leading URI scheme, e.g. `javascript:`, `data:`, `http:`.
// Per RFC 3986 a scheme is ALPHA *( ALPHA / DIGIT / "+" / "-" / "." ).
const SCHEME_RE = /^[a-z][a-z0-9+\-.]*:/i;

/**
 * Returns the `returnTo` value iff it is a safe same-origin relative
 * path; otherwise returns `'/'`. A value is accepted only when ALL of
 * the following hold:
 *
 *   - it is a non-empty string
 *   - it starts with a single `/` (a root-relative path)
 *   - it is NOT protocol-relative (`//host` → would resolve off-origin)
 *   - it has NO URI scheme (`javascript:` / `data:` / `http:` / …)
 *   - it contains NO backslash (`\` is normalised to `/` by browsers,
 *     so `/\evil.com` and `\\evil.com` are off-origin escape vectors)
 *   - its path is NOT under `/auth/` (re-entering the auth surface
 *     after re-auth would loop the user back to signin)
 *   - it is not the bare root `/` (nothing to return *to* — fold to
 *     home, which IS `/`)
 *
 * Everything else (absolute URLs, scheme URIs, protocol-relative
 * `//host`, paths that don't start with `/`, auth surfaces) folds to
 * `'/'`. Query string and fragment are preserved on accepted paths.
 */
export function safeReturnToOrHome(returnTo: string | null | undefined): string {
  if (typeof returnTo !== "string" || returnTo.length === 0) return "/";
  // Must be root-relative. Rejects `app/x` (no leading slash) and,
  // crucially, `//evil.com` (protocol-relative, resolves off-origin).
  if (!returnTo.startsWith("/")) return "/";
  if (returnTo.startsWith("//")) return "/";
  // A backslash anywhere is an off-origin escape vector — browsers
  // treat `\` as `/`, so `/\evil.com` becomes `//evil.com`.
  if (returnTo.includes("\\")) return "/";
  // Any URI scheme (`javascript:`, `data:`, `http:`, …) is rejected.
  // (A safe relative path can still legitimately contain `:` later —
  // e.g. `/x?t=a:b` — so test the scheme only at the start.)
  if (SCHEME_RE.test(returnTo)) return "/";

  const pathOnly = returnTo.split("?", 1)[0]!.split("#", 1)[0]!;
  // Re-entering the auth surface after re-auth loops the user back to
  // signin; fold `/auth` and anything under `/auth/` to home.
  if (pathOnly === "/auth" || pathOnly.startsWith("/auth/")) return "/";
  // The bare root carries no destination — it already IS home.
  if (pathOnly === "/") return "/";

  return returnTo;
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
