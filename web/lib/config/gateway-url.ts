/**
 * Single source of truth for the SKYCOT gateway URL.
 *
 * Every place that emits or consults a gateway URL — the CSP
 * `connect-src` directive in `next.config.ts`, every SPA fetch wrapper,
 * every Next route-handler proxy, every NextAuth callback — MUST read
 * from this module. The historical bug: each consumer hard-coded its
 * own `DEFAULT_GATEWAY = "http://127.0.0.1:7400"` fallback. When one
 * fallback drifted away from the CSP-baked URL, the browser silently
 * blocked every fetch with only a small console line. For production
 * builds the CSP is baked at build time and the only signal is the
 * page appearing broken — a P1 deploy hazard. See
 * `streams/fix-csp-connect-src-gateway-port-drift.md`.
 *
 * Resolution order (matches Next's exposure semantics):
 *
 * 1. `NEXT_PUBLIC_GATEWAY_URL` — set by the operator in
 *    `.env.local` / `.env.production`; visible both server- and
 *    client-side after a Next build.
 * 2. `NEXT_PUBLIC_API_URL` — historical alias still set in some
 *    deploys.
 * 3. `CANONICAL_LOCAL_GATEWAY_URL` — the documented localhost dev
 *    fallback (port 7400 per `CLAUDE.md`).
 *
 * The URL is resolved at call time (not module-load time) so unit
 * tests that mutate `process.env` in `beforeEach` see the updated
 * value, and so the runtime banner can re-probe after a soft env
 * reload during dev.
 *
 * Server-only escape hatches (`GATEWAY_URL` env var) live next to
 * the routes that need them (`web/app/api/auth/demo-mint/route.ts`);
 * they intentionally consult this module's `resolveGatewayUrl()` for
 * the public fallback so the build-time guard stays single-source.
 */

/** Canonical localhost dev port per `CLAUDE.md`. */
export const CANONICAL_LOCAL_GATEWAY_URL = "http://127.0.0.1:7400" as const;

/**
 * Strip a trailing slash so callers can compose with `${URL}/path`
 * without producing `//path`. Idempotent on URLs that already end
 * cleanly.
 */
export function normaliseGatewayUrl(url: string): string {
  return url.replace(/\/+$/, "");
}

/**
 * Resolve the active gateway URL by consulting `process.env` at call
 * time. Reads the same env vars Next exposes to both server and
 * client (`NEXT_PUBLIC_*`), so this resolver returns the same value
 * at SSR, route-handler, and browser-bundle evaluation time. Trailing
 * slashes are stripped.
 */
export function resolveGatewayUrl(): string {
  return normaliseGatewayUrl(
    process.env.NEXT_PUBLIC_GATEWAY_URL ??
      process.env.NEXT_PUBLIC_API_URL ??
      CANONICAL_LOCAL_GATEWAY_URL,
  );
}

/**
 * `true` when the active URL matches the documented dev fallback.
 * Used by the build-time guard to flag silent drift in production
 * deploys, and by the runtime health probe to decide whether a
 * fetch failure is "operator forgot to start the gateway" vs
 * "something deeper is broken".
 */
export function isCanonicalLocalGateway(url: string = resolveGatewayUrl()): boolean {
  return normaliseGatewayUrl(url) === CANONICAL_LOCAL_GATEWAY_URL;
}

/** Build the absolute `${base}/health` URL (server-to-server use). */
export function gatewayHealthUrl(base: string = resolveGatewayUrl()): string {
  return `${normaliseGatewayUrl(base)}/health`;
}

/**
 * The URL the boot-time `GatewayHealthProbe` should fetch.
 *
 * In the browser this returns the SAME-ORIGIN path `/health`, which
 * Next's server-side rewrite (see `next.config.ts`) proxies through to
 * `${gateway}/health`. This mirrors `gatewayBase()` in
 * `lib/api/client.ts`: browser fetches NEVER cross origins (so the
 * CSP `connect-src` / CORS boundary is never exercised), which means
 * the probe reflects the health of the EXACT mechanism the real data
 * path uses — the Next proxy — rather than a direct cross-origin
 * `:7400` fetch the browser refuses (no `Access-Control-Allow-Origin`
 * on the Rust gateway), which produced a false "Gateway unreachable"
 * banner.
 *
 * Server-side (SSR / route handlers) there is no same-origin proxy, so
 * this falls back to the absolute gateway `/health` URL.
 */
export function gatewayHealthProbeUrl(): string {
  if (typeof window !== "undefined") return "/health";
  return gatewayHealthUrl();
}
