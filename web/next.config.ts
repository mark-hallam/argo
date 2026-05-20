import type { NextConfig } from "next";

const gatewayUrl =
  process.env.NEXT_PUBLIC_GATEWAY_URL ??
  process.env.NEXT_PUBLIC_API_URL ??
  "http://127.0.0.1:7400";

const config: NextConfig = {
  output: "standalone",
  poweredByHeader: false,
  reactStrictMode: true,
  typedRoutes: true,
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
  // Permissions-Policy — denies camera/microphone/geolocation by
  //   default. Computer-use is gateway-side, not browser-side.
  // CSP — kept minimal-but-real: 'self' default, gateway URL allowed
  //   for fetch (connect-src), Google Fonts allowed for the Audiowide
  //   wordmark + JetBrains Mono. 'unsafe-inline' on style-src is
  //   required by Next.js styled-jsx + Tailwind preflight; revisit
  //   once we move to CSS-modules or nonces.
  async headers() {
    const gatewayConnect = gatewayUrl.replace(/\/$/, "");
    const csp = [
      "default-src 'self'",
      `connect-src 'self' ${gatewayConnect} https://api.skycot.com`,
      "img-src 'self' data: blob: https:",
      "font-src 'self' https://fonts.gstatic.com data:",
      "style-src 'self' 'unsafe-inline' https://fonts.googleapis.com",
      "script-src 'self' 'unsafe-inline' 'unsafe-eval'",
      "frame-ancestors 'none'",
      "base-uri 'self'",
      "form-action 'self'",
    ].join("; ");
    return [
      {
        source: "/:path*",
        headers: [
          { key: "Strict-Transport-Security", value: "max-age=63072000; includeSubDomains; preload" },
          { key: "X-Frame-Options", value: "DENY" },
          { key: "X-Content-Type-Options", value: "nosniff" },
          { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
          { key: "Permissions-Policy", value: "camera=(), microphone=(), geolocation=()" },
          { key: "Content-Security-Policy", value: csp },
        ],
      },
    ];
  },
};

export default config;
