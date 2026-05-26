import { NextResponse, type NextRequest } from "next/server";

function readSessionCookie(req: NextRequest): string | undefined {
  return (
    req.cookies.get("__Host-session")?.value ??
    req.cookies.get("authjs.session-token")?.value ??
    req.cookies.get("__Secure-authjs.session-token")?.value
  );
}

/**
 * Auth gate. Unauth users hitting any non-`/auth/*` route are
 * redirected to `/auth/signin?from=<original-path>`.
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
    pathname.startsWith("/marketing") ||
    pathname.startsWith("/pricing") ||
    pathname.startsWith("/customers") ||
    pathname.startsWith("/compare") ||
    pathname.startsWith("/changelog") ||
    pathname.startsWith("/docs") ||
    pathname.startsWith("/help") ||
    pathname.startsWith("/sla") ||
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
    return NextResponse.next();
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
    return NextResponse.next();
  }

  if (sessionCookie) return NextResponse.next();

  const signinUrl = req.nextUrl.clone();
  signinUrl.pathname = "/auth/signin";
  signinUrl.search = `?from=${encodeURIComponent(pathname + search)}`;
  return NextResponse.redirect(signinUrl);
}

export const config = {
  matcher: ["/((?!_next/static|_next/image|favicon.ico).*)"],
};
