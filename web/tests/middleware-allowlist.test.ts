import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { NextRequest } from "next/server";
import { middleware } from "../middleware";

const ORIGINAL_NODE_ENV = process.env.NODE_ENV;
const ORIGINAL_BYPASS = process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
const ORIGINAL_DEMO_MODE = process.env.NEXT_PUBLIC_ARGO_DEMO_MODE;

function makeRequest(url: string) {
  return new NextRequest(new URL(url, "http://localhost:3000"));
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
      "http://localhost:3000/auth/signin?from=%2Fdashboard",
    );
  });
});
