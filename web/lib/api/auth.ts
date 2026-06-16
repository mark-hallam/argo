/**
 * Client helpers for the SPA auth endpoints.
 *
 * Exposes `signOut`, the SPA-side wire for the SAME-ORIGIN
 * `POST /api/auth/logout` Next route (`app/api/auth/logout/route.ts`).
 *
 * Why same-origin and not the gateway directly: the `__Host-session`
 * cookie is set on the SPA origin (app.skycot.com) by
 * `/api/auth/verify`. A direct fetch to the gateway's
 * `POST /api/v1/auth/logout` is CROSS-ORIGIN (api.skycot.com), so the
 * browser never sends the app-origin cookie and the credentialed call
 * is CORS-rejected — logout failed and the cookie survived. The
 * same-origin route clears the cookie on THIS origin (and best-effort
 * revokes the JWT `jti` server-to-server). See that route's docstring.
 *
 * The function returns a discriminated `SignOutResult` so callers can
 * decide whether to surface a success toast, redirect, or retry. The
 * same-origin route returns 200 once the cookie is cleared; the 401
 * branch is kept defensively (treated as success — already logged
 * out). Only a network failure reaching the route yields `ok: false`.
 */

import { ApiError } from "./types";

// Same-origin Next route — NOT the cross-origin gateway. A relative URL
// keeps the request on app.skycot.com so the browser sends (and the
// route can clear) the `__Host-session` / `__Host-csrf` cookies.
const LOGOUT_PATH = "/api/auth/logout";

export type SignOutOutcome = "ok" | "already_revoked";

export interface SignOutSuccess {
  ok: true;
  outcome: SignOutOutcome;
}

export interface SignOutFailure {
  ok: false;
  /** HTTP status that came back (or `0` for a network failure). */
  status: number;
  error: ApiError | Error;
}

export type SignOutResult = SignOutSuccess | SignOutFailure;

/**
 * Posts to the same-origin `/api/auth/logout` route with
 * `credentials: "include"` so the browser sends the app-origin
 * `__Host-session` / `__Host-csrf` cookies and accepts the
 * `Set-Cookie: …=; Max-Age=0` clear directives coming back.
 *
 * - 2xx → `{ ok: true, outcome: "ok" }` (cookies cleared on this origin;
 *   the route already best-effort revoked the JWT `jti`).
 * - 401 → `{ ok: true, outcome: "already_revoked" }`. Defensive: treated
 *   as success — indistinguishable from "logged out".
 * - network error reaching the route → `{ ok: false, ... }`. The caller
 *   surfaces a retry prompt.
 */
export async function signOut(): Promise<SignOutResult> {
  let res: Response;
  try {
    res = await fetch(LOGOUT_PATH, {
      method: "POST",
      credentials: "include",
      headers: { Accept: "application/json" },
    });
  } catch (err) {
    return {
      ok: false,
      status: 0,
      error: err instanceof Error ? err : new Error(String(err)),
    };
  }

  if (res.ok) return { ok: true, outcome: "ok" };
  if (res.status === 401) return { ok: true, outcome: "already_revoked" };

  const body = await res.text().catch(() => "");
  return {
    ok: false,
    status: res.status,
    error: new ApiError(res.status, res.statusText || `HTTP ${res.status}`, undefined, body || undefined),
  };
}
