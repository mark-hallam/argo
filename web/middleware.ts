import { NextResponse, type NextRequest } from "next/server";
import { SESSION_MARKER_COOKIE } from "@/lib/skycot/demo-mode";

/**
 * Stamp a readable `skycot-session=1` companion cookie reflecting whether
 * the httpOnly `__Host-session` JWT is present, so the client-side
 * `hasRealSession()` (lib/skycot/demo-mode) can gate demo-mode OFF for a
 * real signed-in user. Anonymous demo visitors get no marker → the demo
 * showcase stays on for them; cleared on sign-out / no session.
 */
function withSessionMarker(
  res: NextResponse,
  hasSession: boolean,
): NextResponse {
  if (hasSession) {
    res.cookies.set(SESSION_MARKER_COOKIE, "1", {
      path: "/",
      sameSite: "lax",
      httpOnly: false,
    });
  } else {
    res.cookies.set(SESSION_MARKER_COOKIE, "", { path: "/", maxAge: 0 });
  }
  return res;
}

function readSessionCookie(req: NextRequest): string | undefined {
  return (
    req.cookies.get("__Host-session")?.value ??
    req.cookies.get("authjs.session-token")?.value ??
    req.cookies.get("__Secure-authjs.session-token")?.value
  );
}

/**
 * Auth gate. Unauth users hitting any non-`/auth/*` route are
 * redirected to `/auth/signin?returnTo=<original-path>`.
 *
 * The param key MUST stay `returnTo` — that is the single key the
 * signin page (`app/auth/signin/page.tsx`) and `parseReturnTo`
 * (`lib/auth/return-to.ts`) read. A prior version emitted `?from=`,
 * which `parseReturnTo` ignored, so the deep-link destination was
 * silently dropped and every post-login landed on `/`. Pinned by
 * `middleware-returnto-param.test.ts`.
 *
 * Dev bypass: setting `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` short-
 * circuits the redirect for local/demo runs. Hard-disabled in
 * production builds — the env var is ignored when
 * `NODE_ENV === "production"`.
 */
export function middleware(req: NextRequest) {
  const { pathname, search } = req.nextUrl;

  if (
    pathname.startsWith("/auth") ||
    pathname.startsWith("/api/auth") ||
    pathname.startsWith("/_next") ||
    pathname === "/favicon.ico" ||
    pathname.startsWith("/landing") ||
    // NOTE: `/marketing` is intentionally NOT allowlisted. `(app)/marketing`
    // is the internal operator-facing GTM console for Marcus (DEC-028) — it
    // must be cookie-gated like the rest of the `(app)/` group. It was wrongly
    // added to the public allowlist on 2026-05-20 (commit 75b73e369) alongside
    // the genuinely-public footer pages; removed here (sprint-ws5-2). The
    // public marketing vertical landing lives at `/for/marketing`, which is
    // covered by the `/for/` prefix below.
    pathname.startsWith("/teammates") ||
    pathname.startsWith("/pricing") ||
    pathname.startsWith("/customers") ||
    pathname.startsWith("/compare") ||
    pathname.startsWith("/changelog") ||
    pathname.startsWith("/docs") ||
    pathname.startsWith("/help") ||
    pathname.startsWith("/sla") ||
    pathname.startsWith("/privacy") ||
    pathname.startsWith("/terms") ||
    pathname.startsWith("/security") ||
    pathname.startsWith("/dpa") ||
    pathname.startsWith("/about") ||
    pathname.startsWith("/contact") ||
    pathname.startsWith("/features") ||
    pathname.startsWith("/blog") ||
    pathname.startsWith("/for/") ||
    pathname === "/opengraph-image" ||
    pathname === "/icon" ||
    pathname === "/apple-icon"
  ) {
    return NextResponse.next();
  }

  // Cookie-name precedence (Wave-11 audit P0 fix, 2026-05-21):
  // canonical is `__Host-session` — the gateway mints it on the
  // SPA edge `/api/auth/verify` proxy after magic-link verify
  // (see `crates/argo-cli/src/handlers/gateway/session_jwt.rs`).
  // Prior middleware checked only `authjs.session-token` /
  // `__Secure-authjs.session-token` — those NextAuth-style names
  // were never minted by the gateway, so every signed-in user
  // bounced back to /auth/signin forever. Read `__Host-session`
  // first; keep the NextAuth names as belt-and-braces fallback.
  const sessionCookie = readSessionCookie(req);

  if (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
  ) {
    return withSessionMarker(NextResponse.next(), Boolean(sessionCookie));
  }

  if (pathname === "/" && !sessionCookie) {
    return NextResponse.redirect(new URL("/landing", req.url));
  }

  // demo-ops-add-magic-link-bypass-for-recording: when the gateway
  // booted with ARGO_DEMO_MODE=1, the SPA mirror NEXT_PUBLIC_ARGO_DEMO_MODE=1
  // short-circuits the cookie gate so the operator can record the
  // single-take demo without an SMTP round-trip. Hard-disabled in
  // production builds (mirrors the auth-bypass branch above).
  if (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_ARGO_DEMO_MODE === "1"
  ) {
    return withSessionMarker(NextResponse.next(), Boolean(sessionCookie));
  }

  if (sessionCookie) return withSessionMarker(NextResponse.next(), true);

  const signinUrl = req.nextUrl.clone();
  signinUrl.pathname = "/auth/signin";
  signinUrl.search = `?returnTo=${encodeURIComponent(pathname + search)}`;
  return withSessionMarker(NextResponse.redirect(signinUrl), false);
}

export const config = {
  matcher: ["/((?!_next/static|_next/image|favicon.ico).*)"],
};
