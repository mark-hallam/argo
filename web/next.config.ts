import type { NextConfig } from "next";

const gatewayUrl =
  process.env.NEXT_PUBLIC_GATEWAY_URL ??
  process.env.NEXT_PUBLIC_API_URL ??
  "http://127.0.0.1:8787";

const config: NextConfig = {
  output: "standalone",
  poweredByHeader: false,
  reactStrictMode: true,
  typedRoutes: true,
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
};

export default config;
