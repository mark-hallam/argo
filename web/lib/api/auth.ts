/**
 * Client helpers for the gateway's auth endpoints.
 *
 * Right now this just exposes `signOut`, the SPA-side wire for
 * `POST /api/v1/auth/logout` (sibling brief #1968). The gateway
 * decodes the `__Host-session` cookie, writes the JWT's `jti` claim
 * into `revoked_tokens`, and emits a `Set-Cookie: __Host-session=;
 * Max-Age=0; ...` header so the browser drops the cookie immediately.
 *
 * The function returns a discriminated `SignOutResult` so callers can
 * decide whether to surface a success toast, redirect, or hold cookies
 * for retry. A 401 is treated as success because the only reason the
 * gateway returns 401 here is that the cookie is already missing or
 * already revoked — both indistinguishable from "logged out" from the
 * user's perspective.
 */

import { ApiError } from "./types";

const LOGOUT_PATH = "/api/v1/auth/logout";
const DEFAULT_GATEWAY = "http://127.0.0.1:7400";

function gatewayBase(): string {
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
}

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
 * Posts to the gateway's logout endpoint with `credentials: "include"`
 * so the browser sends the `__Host-session` cookie and accepts the
 * `Set-Cookie: __Host-session=; Max-Age=0` clear directive coming back.
 *
 * - 2xx → `{ ok: true, outcome: "ok" }`.
 * - 401 → `{ ok: true, outcome: "already_revoked" }`. The gateway only
 *   returns 401 here when the cookie was missing/invalid/already
 *   revoked, which is indistinguishable from "logged out" — UI treats
 *   it as success and redirects.
 * - 5xx / network error → `{ ok: false, ... }`. The caller should keep
 *   the cookie and prompt the user to retry.
 */
export async function signOut(): Promise<SignOutResult> {
  const url = `${gatewayBase()}${LOGOUT_PATH}`;
  let res: Response;
  try {
    res = await fetch(url, {
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
