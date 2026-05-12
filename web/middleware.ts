import { NextResponse, type NextRequest } from "next/server";

/**
 * Auth gate. Unauth users hitting any non-`/auth/*` route are
 * redirected to `/auth/signin?from=<original-path>`.
 *
 * Dev bypass: setting `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` short-
 * circuits the redirect, mirroring the gateway's
 * `ARGO_GATEWAY_AUTH_BYPASS=1` precedent. Hard-disabled in
 * production builds — the env var is ignored when
 * `NODE_ENV === "production"`.
 */
export function middleware(req: NextRequest) {
  const { pathname, search } = req.nextUrl;

  if (
    pathname.startsWith("/auth") ||
    pathname.startsWith("/api/auth") ||
    pathname.startsWith("/_next") ||
    pathname === "/favicon.ico"
  ) {
    return NextResponse.next();
  }

  if (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS === "1"
  ) {
    return NextResponse.next();
  }

  const sessionCookie =
    req.cookies.get("authjs.session-token")?.value ??
    req.cookies.get("__Secure-authjs.session-token")?.value;

  if (sessionCookie) return NextResponse.next();

  const signinUrl = req.nextUrl.clone();
  signinUrl.pathname = "/auth/signin";
  signinUrl.search = `?from=${encodeURIComponent(pathname + search)}`;
  return NextResponse.redirect(signinUrl);
}

export const config = {
  matcher: ["/((?!_next/static|_next/image|favicon.ico).*)"],
};
