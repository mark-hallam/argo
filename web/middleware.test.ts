import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NextRequest } from "next/server";
import { middleware } from "./middleware";
import { parseReturnTo } from "@/lib/auth/return-to";

const ORIGINAL_NODE_ENV = process.env.NODE_ENV;
const ORIGINAL_BYPASS = process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;

function makeRequest(url: string, { withSession }: { withSession?: boolean } = {}) {
  const req = new NextRequest(new URL(url, "http://localhost:3000"));
  if (withSession) {
    req.cookies.set("authjs.session-token", "test-session-cookie");
  }
  return req;
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
}

describe("middleware", () => {
  beforeEach(() => {
    Object.defineProperty(process.env, "NODE_ENV", {
      configurable: true,
      enumerable: true,
      writable: true,
      value: "test",
    });
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
  });
  afterEach(() => {
    vi.restoreAllMocks();
    restoreEnv();
  });

  it("redirects unauth requests to /auth/signin with returnTo", () => {
    const res = middleware(makeRequest("http://localhost:3000/team"));
    expect(res.status).toBe(307);
    const loc = res.headers.get("location") ?? "";
    expect(loc).toContain("/auth/signin");
    expect(loc).toContain(`returnTo=${encodeURIComponent("/team")}`);
    // The old `?from=` key (which `parseReturnTo` never read, so the
    // destination was dropped) must NOT be emitted.
    expect(loc).not.toContain("from=");
  });

  // Param-key parity: the key the middleware emits MUST be the exact key
  // `parseReturnTo` reads — otherwise the deep-link destination is
  // silently dropped and every post-login lands on `/` (the bug this
  // fixes). We round-trip a redirect's query through `parseReturnTo` and
  // assert it recovers the original path including its query string.
  it("emits a returnTo key that parseReturnTo reads back (no param drift)", () => {
    const res = middleware(
      makeRequest("http://localhost:3000/workers/abc?tab=memory"),
    );
    expect(res.status).toBe(307);
    const loc = res.headers.get("location") ?? "";
    const search = new URL(loc, "http://localhost:3000").searchParams;
    expect(parseReturnTo(search)).toBe("/workers/abc?tab=memory");
  });

  it("lets requests through when a session cookie is present", () => {
    const res = middleware(makeRequest("http://localhost:3000/", { withSession: true }));
    expect(res.headers.get("location")).toBeNull();
  });

  it("lets requests through when bypass flag is set in non-prod", () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    const res = middleware(makeRequest("http://localhost:3000/"));
    expect(res.headers.get("location")).toBeNull();
  });

  it("ignores bypass flag in production", () => {
    Object.defineProperty(process.env, "NODE_ENV", {
      configurable: true,
      enumerable: true,
      writable: true,
      value: "production",
    });
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    const res = middleware(makeRequest("http://localhost:3000/"));
    expect(res.status).toBe(307);
  });

  it("never redirects /auth/* routes", () => {
    const res = middleware(makeRequest("http://localhost:3000/auth/signin"));
    expect(res.headers.get("location")).toBeNull();
  });

  it.each(["/opengraph-image", "/icon", "/apple-icon"])(
    "lets the metadata route %s through unauthenticated",
    (path) => {
      const res = middleware(makeRequest(`http://localhost:3000${path}`));
      expect(res.headers.get("location")).toBeNull();
    },
  );

  it.each([
    "/landing/opengraph-image",
    "/pricing/opengraph-image",
    "/compare/opengraph-image",
    "/customers/opengraph-image",
  ])("lets per-page metadata route %s through unauthenticated", (path) => {
    const res = middleware(makeRequest(`http://localhost:3000${path}`));
    expect(res.headers.get("location")).toBeNull();
  });
});
