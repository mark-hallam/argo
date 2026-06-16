import type { NextConfig } from "next";

// The build-time gateway URL MUST stay in lock-step with the
// canonical constant exported by `web/lib/config/gateway-url.ts`.
// Both must point at the same host or the CSP `connect-src`
// directive baked into HTML at build time will refuse the runtime
// fetches that the SPA hooks issue, and the only signal a buyer
// would get is a small browser-console line on a broken page.
// The `prebuild` script `web/scripts/check-canonical-gateway-port.mjs`
// reads `NEXT_PUBLIC_GATEWAY_URL` and the constant and refuses to
// ship if they have drifted. Keep this literal in sync with
// `CANONICAL_LOCAL_GATEWAY_URL` in that module.
const CANONICAL_LOCAL_GATEWAY_URL = "http://127.0.0.1:7400";

const gatewayUrl =
  process.env.NEXT_PUBLIC_GATEWAY_URL ??
  process.env.NEXT_PUBLIC_API_URL ??
  CANONICAL_LOCAL_GATEWAY_URL;

const config: NextConfig = {
  output: "standalone",
  // Per-page static-generation timeout (default 60s). The 5 opengraph-image
  // routes render via @vercel/og (Satori + resvg-WASM), which is fine (<5s) on a
  // native builder but exceeds 60s under linux/amd64 QEMU emulation (e.g. building
  // the deploy image on an Apple-Silicon host), failing the build after 3 retries.
  // 600s lets a slow/emulated builder finish; fast native builds are unaffected.
  staticPageGenerationTimeout: 600,
  poweredByHeader: false,
  reactStrictMode: true,
  typedRoutes: true,
  // Canonical paths are no-slash. Without this, Next.js still strips
  // a trailing slash but does it via a 308 redirect on cold visits,
  // adding a hop (~50ms TTFB) for shared links like `/landing/`.
  // Explicit `false` short-circuits the redirect for the canonical
  // form and keeps non-canonical forms a single 308.
  trailingSlash: false,
  productionBrowserSourceMaps: process.env.CI === "true",
  images: {
    remotePatterns: [
      { protocol: "https", hostname: "api.skycot.com" },
      { protocol: "https", hostname: "try.skycot.com" },
    ],
  },
  env: {
    NEXT_PUBLIC_GATEWAY_URL: gatewayUrl,
    NEXT_PUBLIC_API_URL: process.env.NEXT_PUBLIC_API_URL ?? gatewayUrl,
  },
  // Dev-mode SPA→gateway proxy. In production, SPA and gateway are
  // colocated behind a single load balancer so this rewrite is a no-op
  // (same-origin). In dev they're on different ports (3000 vs 7400),
  // so direct cross-origin fetch fails CORS preflight (the Rust gateway
  // has no OPTIONS handler). Proxying server-side through Next.js
  // avoids CORS entirely.
  async rewrites() {
    return [
      { source: "/api/v1/:path*", destination: `${gatewayUrl}/api/v1/:path*` },
      // Same-origin gateway-liveness passthrough. The gateway exposes
      // its combined health probe at the ROOT (`/health`), not under
      // `/api/v1/*`. The boot-time `GatewayHealthProbe` must reflect the
      // health of the REAL data path — which is this server-side Next
      // proxy, not a direct browser fetch. A direct cross-origin
      // `http://127.0.0.1:7400/health` fetch from the `:3000` SPA is
      // refused by the browser (the Rust gateway sends no
      // `Access-Control-Allow-Origin` header and has no OPTIONS handler),
      // so the probe used to render a false "Gateway unreachable" banner
      // even when the gateway was healthy and every `/api/v1/*` call
      // succeeded. Routing the probe through this same-origin rewrite
      // makes it server-to-server (no browser cross-origin, CSP/CORS
      // boundary untouched) and reports the data path's true health.
      { source: "/health", destination: `${gatewayUrl}/health` },
    ];
  },
  // Bare auth paths → canonical `/auth/*` (308 permanent). Operators or
  // marketing copy may link to `/signup`, `/signin`, `/verify` even
  // though the canonical routes live under `/auth/*`. 308 preserves the
  // HTTP method and is cacheable by the browser.
  async redirects() {
    return [
      { source: "/signup", destination: "/auth/signup", permanent: true },
      { source: "/signin", destination: "/auth/signin", permanent: true },
      { source: "/verify", destination: "/auth/verify", permanent: true },
      // The canvas only exists at the nested per-worker / per-skill
      // path `/workers/[workerId]/skills/[skillId]/canvas`. The flat
      // `/skills-canvas` URL was never built but lingers in audit
      // history + cross-brief references. 307 (not 308) because the
      // marketplace landing may change shape; we don't want browsers
      // caching the redirect permanently.
      { source: "/skills-canvas", destination: "/skills", permanent: false },
      // Settings stub-route redirects (P1 fix, UX-AUDIT-T0119MEL F12).
      // The operator's mental model expects `branding`, `integrations`,
      // and `danger` as settings URLs, but the locked D design lives
      // them under existing tabs: appearance under `/settings/workspace`,
      // connectors as the integration surface, and Danger zone as the
      // `#danger-zone` anchor that the settings layout renders on every
      // sub-page. 307 (not 308) because the destination layout may
      // evolve before v1.0; we don't want browsers caching the redirect
      // permanently.
      {
        source: "/settings/branding",
        destination: "/settings/workspace#appearance",
        permanent: false,
      },
      {
        source: "/settings/integrations",
        destination: "/settings/connectors",
        permanent: false,
      },
      {
        source: "/settings/danger",
        destination: "/settings/workspace#danger-zone",
        permanent: false,
      },
      // Dead-route catchers. `/workflows/runs` and `/workflows/edit` were
      // never built — the canonical surfaces are `/audit` (with the
      // `workflow.execute` action filter) for run history, and the
      // `/workflows` catalog → per-workflow `[verticalId]/[name]/build`
      // path for editing. Stale prototype links, browser bookmarks, and
      // Next.js prerender attempts were producing 404 noise in dev logs
      // (UX-AUDIT-T0519 F11). `scripts/check-no-dead-spa-routes.sh`
      // prevents new in-product links to these paths; the redirects
      // catch anything in-the-wild.
      {
        source: "/workflows/runs",
        destination: "/audit?range=7d&action_type=workflow.execute",
        permanent: false,
      },
      { source: "/workflows/edit", destination: "/workflows", permanent: false },
    ];
  },
  // SECURITY (P1 fix, 2026-05-20): baseline browser security headers.
  // HSTS — Strict-Transport-Security: force HTTPS for 2 years inc.
  //   subdomains. Safe on app.skycot.com / skycot.com which are both
  //   intended to be HTTPS-only in prod.
  // X-Frame-Options DENY — prevents clickjacking by blocking iframe
  //   embedding (even from same origin). We have no in-product iframe
  //   embed today; revisit if marketing embeds the product.
  // X-Content-Type-Options nosniff — stops MIME-sniffing attacks.
  // Referrer-Policy strict-origin-when-cross-origin — leaks origin
  //   only to cross-site requests, never path/query.
  // Permissions-Policy — denies camera/geolocation; allows microphone for
  //   same-origin (self) ONLY, required by the in-browser live-voice feature
  //   (/pitch "Go Live", /coach). Without microphone=(self) the browser blocks
  //   getUserMedia with a Permissions-Policy violation before any user prompt.
  //   Computer-use is gateway-side, not browser-side.
  // CSP — moved to `web/middleware.ts` (2026-06-08). It is now a
  //   PER-REQUEST policy carrying a fresh nonce so `script-src` can drop
  //   'unsafe-inline' (nonce + 'strict-dynamic' instead). A static header
  //   here cannot mint a per-request nonce, and Next.js only nonces its
  //   own framework scripts when it reads the nonce off the REQUEST CSP
  //   header — which only middleware can set. The other (static) security
  //   headers below stay here; only CSP relocated. NB: do NOT re-add a
  //   `Content-Security-Policy` key here — two CSP headers would both be
  //   enforced and the static one (no nonce) would block the framework.
  async headers() {
    const baseHeaders = [
      {
        source: "/:path*",
        headers: [
          { key: "Strict-Transport-Security", value: "max-age=63072000; includeSubDomains; preload" },
          { key: "X-Frame-Options", value: "DENY" },
          { key: "X-Content-Type-Options", value: "nosniff" },
          { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
          { key: "Permissions-Policy", value: "camera=(), microphone=(self), geolocation=()" },
        ],
      },
    ];
    // PERF (P3 fix, 2026-05-21 — web-perf-cache-control-marketing):
    // production-mode Cache-Control for public marketing routes.
    //   public               — any cache may store (browser + CDN).
    //   s-maxage=300         — shared (CDN) caches keep for 5 minutes.
    //   stale-while-revalidate=600 — serve stale up to 10 min while
    //                          refetching in the background.
    // Strategy: marketing copy changes ~daily, not per-second; 5-min
    // edge cache is invisible to operators (CDN purge on deploy
    // handles fresh-pull) while massively reducing origin RPS and
    // satisfying Lighthouse "efficient cache policy".
    // Dev-mode (NODE_ENV !== "production") falls through to Next.js
    // defaults (no-store) so HMR / route-changes reflect immediately.
    // Authed routes (anything not in this matcher) and /api/* keep
    // their default no-store — correct for personalized content.
    if (process.env.NODE_ENV === "production") {
      const marketingPaths = [
        "/landing/:path*",
        "/pricing/:path*",
        "/customers/:path*",
        "/compare/:path*",
        "/changelog/:path*",
        "/docs/:path*",
        "/privacy/:path*",
        "/terms/:path*",
        "/security/:path*",
        "/dpa/:path*",
        "/about/:path*",
        "/features/:path*",
        "/blog/:path*",
        "/for/:path*",
      ];
      for (const source of marketingPaths) {
        baseHeaders.push({
          source,
          headers: [
            {
              key: "Cache-Control",
              value: "public, s-maxage=300, stale-while-revalidate=600",
            },
          ],
        });
      }
    }
    return baseHeaders;
  },
};

export default config;
