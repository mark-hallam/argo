import { NextResponse, type NextRequest } from "next/server";
import { SESSION_MARKER_COOKIE } from "@/lib/skycot/demo-mode";

/**
 * Build the per-request Content-Security-Policy and the random nonce it
 * carries.
 *
 * SECURITY (2026-06-08, nonce-CSP hardening): `script-src` no longer
 * contains `'unsafe-inline'`. Instead each response carries a fresh
 * base64 nonce in a `'nonce-…'` source, plus `'strict-dynamic'` so the
 * trust on Next.js's own nonce'd bootstrap scripts propagates to the
 * chunks they load (without having to enumerate every webpack chunk URL).
 *
 * How Next.js picks the nonce up: when it renders an App-Router page it
 * reads the **request** `content-security-policy` header, finds the
 * `script-src` (or `default-src`) directive, and extracts the first
 * `'nonce-<base64>'` source via the regex
 * `/^'nonce-([A-Za-z0-9+/_-]+={0,2})'$/`
 * (`node_modules/next/dist/server/app-render/get-script-nonce-from-header.js`).
 * It then stamps that nonce onto every framework `<script>` it emits.
 * So the SAME CSP string must be set as BOTH a forwarded request header
 * (so the renderer can read the nonce) AND the response header (so the
 * browser enforces it).
 *
 * `style-src` keeps `'unsafe-inline'` deliberately — Next.js styled-jsx
 * and the Tailwind preflight inject inline `<style>` with no nonce hook,
 * and inline styles are a far weaker XSS vector than inline scripts.
 *
 * The gateway origins (http + ws) and the third-party endpoints the live
 * features need (Google Fonts CSS/woff, OpenAI realtime) are unchanged
 * from the previous static policy in `next.config.ts`.
 */
function buildCsp(): { csp: string; nonce: string } {
  // 16 random bytes → base64. The edge runtime exposes the Web Crypto
  // `crypto` global, so no Node import is needed (middleware runs on the
  // edge runtime). base64 only ever yields chars in `[A-Za-z0-9+/=]`,
  // all of which Next's nonce regex accepts.
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  const nonce = btoa(String.fromCharCode(...bytes));

  // Mirror the gateway/connect origins the SPA actually talks to. These
  // are compile-time-known public env values (same source the old static
  // CSP in next.config.ts used); falling back to the canonical local
  // gateway keeps dev working when the env var is unset.
  const gatewayUrl = (
    process.env.NEXT_PUBLIC_GATEWAY_URL ??
    process.env.NEXT_PUBLIC_API_URL ??
    "http://127.0.0.1:7400"
  ).replace(/\/$/, "");
  // `connect-src` treats http(s): and ws(s): as distinct schemes, so the
  // live approvals inbox WebSocket (`/inbox/ws`) needs the ws-scheme
  // origin whitelisted explicitly or the handshake is silently refused.
  const gatewayWs = gatewayUrl.replace(/^http(s?):/, "ws$1:");

  const csp = [
    "default-src 'self'",
    // `https://api.openai.com` — live-voice WebRTC SDP handshake
    // (/pitch "Go Live", /coach) POSTs to
    // `https://api.openai.com/v1/realtime/calls`.
    `connect-src 'self' ${gatewayUrl} ${gatewayWs} https://api.openai.com https://api.skycot.com wss://api.skycot.com`,
    "img-src 'self' data: blob: https:",
    "font-src 'self' https://fonts.gstatic.com data:",
    "style-src 'self' 'unsafe-inline' https://fonts.googleapis.com",
    // 'unsafe-inline' for SCRIPT is GONE — this is the whole point of the
    // hardening. Next.js applies this nonce to its bootstrap scripts (it
    // reads the nonce off the request CSP header during render), and
    // 'strict-dynamic' lets those nonce-trusted scripts load the rest of
    // the webpack chunk graph without us enumerating every chunk URL.
    // NB: in a CSP3 browser, the presence of a nonce/'strict-dynamic'
    // makes the browser IGNORE 'unsafe-inline' AND host-allowlist sources
    // like 'self'/https:, so adding those would be dead weight — the
    // nonce alone is the gate. 'self' is retained only so legacy CSP1
    // browsers (which ignore the nonce + strict-dynamic) can still load
    // same-origin <script src> chunks rather than hard-blanking.
    `script-src 'self' 'nonce-${nonce}' 'strict-dynamic'`,
    "frame-ancestors 'none'",
    "base-uri 'self'",
    "form-action 'self'",
  ].join("; ");

  return { csp, nonce };
}

/**
 * Attach the CSP to the response, and (on rendered, non-redirect
 * responses) forward the CSP + nonce as REQUEST headers so the Next.js
 * renderer can read the nonce and stamp it on its framework scripts.
 *
 * For redirects there is no rendered HTML, so only the response header is
 * meaningful; we still set it for defence-in-depth on the 3xx body.
 */
function withCsp(
  res: NextResponse,
  csp: string,
): NextResponse {
  res.headers.set("Content-Security-Policy", csp);
  return res;
}

/** Build the forwarded request headers carrying the nonce + CSP. */
function forwardHeaders(req: NextRequest, csp: string, nonce: string): Headers {
  const requestHeaders = new Headers(req.headers);
  requestHeaders.set("x-nonce", nonce);
  // Next.js reads the nonce off the REQUEST `content-security-policy`
  // header during app render — this line is what makes the framework
  // scripts get `nonce="…"`.
  requestHeaders.set("Content-Security-Policy", csp);
  return requestHeaders;
}

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

/**
 * Canonicalize the loopback host to `localhost` BEFORE any cookie read.
 *
 * The session cookie is `__Host-session`, minted with the `__Host-`
 * prefix (`Path=/; HttpOnly; Secure; SameSite=Lax`, no `Domain`), which
 * host-pins it to the exact origin that set it. The mint origin is
 * `localhost:3000` (`app/api/auth/demo-mint/route.ts`). So a cookie set
 * on `localhost` is NOT sent when the app is opened on `127.0.0.1` — the
 * two are different hosts even though both are loopback — and the session
 * silently drops. The visible symptom was an onboarding-save that
 * mislabelled the resulting gateway auth-reject as "demo mode".
 *
 * Fix: when the request arrives on the non-canonical loopback host
 * `127.0.0.1`, 307-redirect to the same URL with the hostname rewritten
 * to `localhost` (port, path, and search preserved). The browser then
 * re-requests on the canonical host and sends the existing
 * `__Host-session` cookie, so the session survives. Returns `null` for
 * every other host (including `localhost` itself — so it never loops, and
 * never touches non-loopback hosts).
 *
 * Exported only so the unit test can exercise it directly: in the vitest
 * harness `NextRequest`'s `nextUrl` normalizes a `127.0.0.1` input back to
 * `localhost` (it derives the host from a default base, ignoring the
 * loopback IP), so the branch is untestable through `middleware()`. The
 * sole *runtime* consumer is still `middleware()` below. The target is
 * built from `nextUrl.href` (not `.clone()`) so any URL-shaped value
 * works.
 */
export function canonicalLoopbackRedirect(req: NextRequest): NextResponse | null {
  if (req.nextUrl.hostname !== "127.0.0.1") return null;
  const target = new URL(req.nextUrl.href);
  target.hostname = "localhost";
  return NextResponse.redirect(target, 307);
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
 *
 * Every response carries a per-request nonce CSP (see `buildCsp`).
 * Rendered responses additionally forward the nonce to the Next.js
 * renderer via request headers.
 */
export function middleware(req: NextRequest) {
  const { pathname, search } = req.nextUrl;
  const { csp, nonce } = buildCsp();

  // Canonicalize the loopback host BEFORE any cookie read, so the
  // `__Host-`-pinned session cookie set on `localhost` survives a visit
  // to `127.0.0.1` (see `canonicalLoopbackRedirect`). Must run first —
  // returning here means the browser re-requests on the canonical host
  // and the cookie-bearing request then flows through the gates below.
  const loopbackRedirect = canonicalLoopbackRedirect(req);
  if (loopbackRedirect) return withCsp(loopbackRedirect, csp);
  // A rendered (pass-through) response forwards the nonce to the renderer.
  const passThrough = () =>
    NextResponse.next({ request: { headers: forwardHeaders(req, csp, nonce) } });

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
    return withCsp(passThrough(), csp);
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
    return withCsp(
      withSessionMarker(passThrough(), Boolean(sessionCookie)),
      csp,
    );
  }

  if (pathname === "/" && !sessionCookie) {
    return withCsp(NextResponse.redirect(new URL("/landing", req.url)), csp);
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
    return withCsp(
      withSessionMarker(passThrough(), Boolean(sessionCookie)),
      csp,
    );
  }

  if (sessionCookie)
    return withCsp(withSessionMarker(passThrough(), true), csp);

  const signinUrl = req.nextUrl.clone();
  signinUrl.pathname = "/auth/signin";
  signinUrl.search = `?returnTo=${encodeURIComponent(pathname + search)}`;
  return withCsp(
    withSessionMarker(NextResponse.redirect(signinUrl), false),
    csp,
  );
}

export const config = {
  matcher: ["/((?!_next/static|_next/image|favicon.ico).*)"],
};
