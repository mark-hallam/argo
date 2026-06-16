import { NextResponse } from "next/server";
import { cookies } from "next/headers";
import { resolveGatewayUrl } from "@/lib/config/gateway-url";

/**
 * `POST /api/auth/logout` — same-origin logout for the split topology.
 *
 * Why this route exists (the bug it fixes): the `__Host-session` (JWT)
 * and `__Host-csrf` cookies are set on the SPA origin (app.skycot.com)
 * by `/api/auth/verify`, which relays the gateway's `Set-Cookie`s. The
 * previous logout (`lib/api/auth.ts`) fetched the gateway's
 * `POST /api/v1/auth/logout` directly — i.e. CROSS-ORIGIN to
 * api.skycot.com. The browser therefore (a) never sends the
 * app-origin `__Host-session` cookie to the api origin, so the gateway
 * can't read or clear it, and (b) the credentialed cross-origin call
 * is itself CORS-rejected → the button surfaced "Couldn't sign out".
 * Net: the session cookie survived and the user could not log out.
 *
 * The cookie lives HERE (app origin), so only THIS origin can clear it.
 * Mirrors the inbound pattern of `/api/principal` + `/api/auth/verify`.
 *
 * Behaviour:
 *  1. Best-effort, server-to-server (no browser CORS): forward the
 *     `__Host-session` cookie to the gateway logout so it records the
 *     JWT `jti` in `revoked_tokens` (defence-in-depth against replay of
 *     a copied token). The gateway logout deliberately skips CSRF
 *     (`routes_auth_logout.rs` — "the way OUT"), so the cookie is all
 *     it needs. A gateway failure must NOT block the local clear.
 *  2. ALWAYS expire both cookies on this origin and return 200. The
 *     local clear is what actually logs the user out, so it must
 *     succeed even when the gateway is unreachable.
 */
export const dynamic = "force-dynamic";

const SESSION_COOKIE = "__Host-session";
const CSRF_COOKIE = "__Host-csrf";

// Mirror the gateway's known-good clear directive (routes_auth_logout.rs
// CLEARED_COOKIE) for the HttpOnly session cookie, plus the parallel
// directive for the non-HttpOnly CSRF cookie. `__Host-` requires
// `Secure` + `Path=/` + no `Domain`; an empty value with `Max-Age=0`
// instructs every conforming browser to drop the cookie immediately.
const CLEAR_SESSION = `${SESSION_COOKIE}=; Max-Age=0; Path=/; HttpOnly; Secure; SameSite=Lax`;
const CLEAR_CSRF = `${CSRF_COOKIE}=; Max-Age=0; Path=/; Secure; SameSite=Lax`;

export async function POST() {
  const store = await cookies();
  const session = store.get(SESSION_COOKIE)?.value;

  if (session) {
    try {
      await fetch(`${resolveGatewayUrl()}/api/v1/auth/logout`, {
        method: "POST",
        headers: {
          Accept: "application/json",
          // The gateway logout reads the session cookie directly; no
          // CSRF header needed (it skips the CSRF gate on this route).
          Cookie: `${SESSION_COOKIE}=${session}`,
        },
        cache: "no-store",
      });
    } catch {
      // Gateway unreachable / revoke failed — intentionally swallowed.
      // The cookie clear below is the load-bearing step; a transient
      // gateway outage must never trap the user in a logged-in state.
    }
  }

  const res = NextResponse.json({ ok: true });
  res.headers.append("set-cookie", CLEAR_SESSION);
  res.headers.append("set-cookie", CLEAR_CSRF);
  return res;
}
