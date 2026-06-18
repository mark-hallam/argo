import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { NextRequest } from "next/server";
import { middleware } from "../middleware";

const ORIGINAL_NODE_ENV = process.env.NODE_ENV;
const ORIGINAL_BYPASS = process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
const ORIGINAL_DEMO_MODE = process.env.NEXT_PUBLIC_ARGO_DEMO_MODE;

function makeRequest(url: string) {
  return new NextRequest(new URL(url, "http://localhost:3000"));
}

function makePost(url: string) {
  return new NextRequest(new URL(url, "http://localhost:3000"), {
    method: "POST",
  });
}

function restoreEnv() {
  Object.defineProperty(process.env, "NODE_ENV", {
    configurable: true,
    enumerable: true,
    writable: true,
    value: ORIGINAL_NODE_ENV,
  });
  if (ORIGINAL_BYPASS === undefined) delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
  else process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = ORIGINAL_BYPASS;
  if (ORIGINAL_DEMO_MODE === undefined) delete process.env.NEXT_PUBLIC_ARGO_DEMO_MODE;
  else process.env.NEXT_PUBLIC_ARGO_DEMO_MODE = ORIGINAL_DEMO_MODE;
}

describe("middleware public-paths allowlist — marketing footer legal links", () => {
  beforeEach(() => {
    Object.defineProperty(process.env, "NODE_ENV", {
      configurable: true,
      enumerable: true,
      writable: true,
      value: "test",
    });
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
    delete process.env.NEXT_PUBLIC_ARGO_DEMO_MODE;
  });

  afterEach(() => {
    restoreEnv();
  });

  // whole-site F01: footer-linked legal pages must render for logged-out
  // visitors. Pre-fix, /privacy /terms /security 307'd to /auth/signin?from=...
  for (const path of ["/privacy", "/terms", "/security"]) {
    it(`lets unauthenticated GET ${path} pass through (no signin redirect)`, () => {
      const res = middleware(makeRequest(`http://localhost:3000${path}`));

      // NextResponse.next() carries no Location header; a redirect would.
      expect(res.headers.get("location")).toBeNull();
    });
  }

  it("still redirects unauthenticated traffic on unlisted paths", () => {
    const res = middleware(makeRequest("http://localhost:3000/dashboard"));

    expect(res.status).toBe(307);
    expect(res.headers.get("location")).toBe(
      "http://localhost:3000/auth/signin?returnTo=%2Fdashboard",
    );
  });
});

// sprint-ws5-2: `(app)/marketing` is the internal operator-facing GTM console
// (Marcus, DEC-028), not a public landing. It was wrongly allowlisted on
// 2026-05-20 (commit 75b73e369) so it rendered without a session. Pin that an
// unauthenticated request now redirects to sign-in, while genuinely-public
// routes (e.g. /pricing, and the public marketing vertical landing
// /for/marketing) still pass through.
describe("middleware auth gate — internal /marketing console is cookie-gated", () => {
  beforeEach(() => {
    Object.defineProperty(process.env, "NODE_ENV", {
      configurable: true,
      enumerable: true,
      writable: true,
      value: "test",
    });
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
    delete process.env.NEXT_PUBLIC_ARGO_DEMO_MODE;
  });

  afterEach(() => {
    restoreEnv();
  });

  it("redirects unauthenticated GET /marketing to sign-in", () => {
    const res = middleware(makeRequest("http://localhost:3000/marketing"));

    expect(res.status).toBe(307);
    expect(res.headers.get("location")).toBe(
      "http://localhost:3000/auth/signin?returnTo=%2Fmarketing",
    );
  });

  it("keeps the public /pricing route reachable without a session", () => {
    const res = middleware(makeRequest("http://localhost:3000/pricing"));

    // NextResponse.next() carries no Location header; a redirect would.
    expect(res.headers.get("location")).toBeNull();
  });

  it("keeps the public /for/marketing vertical landing reachable without a session", () => {
    const res = middleware(makeRequest("http://localhost:3000/for/marketing"));

    expect(res.headers.get("location")).toBeNull();
  });
});

// web-waitlist-failure-ux-and-anon-post-bypass (shakedown F2): the anonymous
// marketing lead-capture POST proxies (`/api/v1/waitlist`,
// `/api/v1/enterprise-inquiry`) are mounted ahead of the gateway's bearer-auth
// gate and are public by design. They MUST pass through to the gateway rewrite,
// never the `/auth/signin?returnTo=%2Fapi%2Fv1%2Fwaitlist` chain — bouncing an
// anonymous POST into signin silently loses the lead. The exemption is scoped
// to those exact paths; the rest of `/api/v1/*` stays cookie-gated.
describe("middleware — anonymous marketing POST proxies bypass the signin redirect", () => {
  beforeEach(() => {
    Object.defineProperty(process.env, "NODE_ENV", {
      configurable: true,
      enumerable: true,
      writable: true,
      value: "test",
    });
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
    delete process.env.NEXT_PUBLIC_ARGO_DEMO_MODE;
  });

  afterEach(() => {
    restoreEnv();
  });

  for (const path of ["/api/v1/waitlist", "/api/v1/enterprise-inquiry"]) {
    it(`lets an unauthenticated POST ${path} pass through (no signin redirect)`, () => {
      const res = middleware(makePost(`http://localhost:3000${path}`));

      // NextResponse.next() carries no Location header; a redirect would.
      expect(res.headers.get("location")).toBeNull();
    });
  }

  it("still redirects an unauthenticated POST to a non-exempt /api/v1 route", () => {
    const res = middleware(makePost("http://localhost:3000/api/v1/workers"));

    expect(res.status).toBe(307);
    expect(res.headers.get("location")).toBe(
      "http://localhost:3000/auth/signin?returnTo=%2Fapi%2Fv1%2Fworkers",
    );
  });
});

// fix-spa-middleware-redirects-public-api-routes: genuinely-public gateway API
// routes proxied via the `/api/v1/:path*` rewrite must reach the gateway, not
// the HTML signin page — bouncing an anonymous XHR into /auth/signin returns
// HTML where JSON is expected and breaks the surface (e.g. error-reporting
// stayed broken in-browser even after it was made public gateway-side in
// #2984). Mirrors the gateway's `is_public_get` allowlist + the public-pitch
// POST front door. Pinned as EXACT matches so the tenant-scoped neighbours
// (`/api/v1/skills`, the rest of `/api/v1/meta/*`, the authed `/api/v1/pitch/ask`)
// stay cookie-gated.
describe("middleware — public gateway API routes bypass the signin redirect", () => {
  beforeEach(() => {
    Object.defineProperty(process.env, "NODE_ENV", {
      configurable: true,
      enumerable: true,
      writable: true,
      value: "test",
    });
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
    delete process.env.NEXT_PUBLIC_ARGO_DEMO_MODE;
  });

  afterEach(() => {
    restoreEnv();
  });

  for (const path of [
    "/api/v1/meta/error-reporting",
    "/api/v1/skills/marketplace",
    "/api/v1/skill-library/list",
  ]) {
    it(`lets an unauthenticated GET ${path} pass through (no signin redirect)`, () => {
      const res = middleware(makeRequest(`http://localhost:3000${path}`));

      // NextResponse.next() carries no Location header; a redirect would.
      expect(res.headers.get("location")).toBeNull();
    });
  }

  it("lets an unauthenticated POST /api/v1/pitch/ask/public pass through (no signin redirect)", () => {
    const res = middleware(makePost("http://localhost:3000/api/v1/pitch/ask/public"));

    expect(res.headers.get("location")).toBeNull();
  });

  // Negative pins: the exact-match additions must NOT open the tenant-scoped
  // neighbours. Each of these still bounces an anonymous caller to signin.
  for (const path of [
    "/api/v1/skills",
    "/api/v1/meta/billing",
    "/api/v1/pitch/ask",
  ]) {
    it(`still redirects an unauthenticated GET ${path} (exact-match did not open the prefix)`, () => {
      const res = middleware(makeRequest(`http://localhost:3000${path}`));

      expect(res.status).toBe(307);
      expect(res.headers.get("location")).toBe(
        `http://localhost:3000/auth/signin?returnTo=${encodeURIComponent(path)}`,
      );
    });
  }
});
