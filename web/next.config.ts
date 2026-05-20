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
};

export default config;
