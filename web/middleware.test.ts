import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NextRequest } from "next/server";
import { canonicalLoopbackRedirect, middleware } from "./middleware";
import { parseReturnTo } from "@/lib/auth/return-to";
import { SESSION_MARKER_COOKIE } from "@/lib/skycot/demo-mode";

// `NextRequest`'s `nextUrl` normalizes a `127.0.0.1` input back to
// `localhost` inside the vitest harness, so we exercise the loopback
// branch through a minimal stub whose `nextUrl` is a real URL pinned to
// the loopback IP — exactly what a real Next.js server builds from the
// `Host: 127.0.0.1:3000` request header.
function loopbackReq(path: string): NextRequest {
  return { nextUrl: new URL(`http://127.0.0.1:3000${path}`) } as unknown as NextRequest;
}

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

  // Loopback-host canonicalization (sprint-ws1): the `__Host-session`
  // cookie is host-pinned to `localhost`, so a visit to `127.0.0.1` drops
  // the session. The front door redirects the non-canonical loopback host
  // to `localhost` BEFORE any cookie read so the session survives.
  describe("loopback-host canonicalization", () => {
    it("307-redirects 127.0.0.1 to localhost, preserving port/path/search", () => {
      const res = canonicalLoopbackRedirect(
        loopbackReq("/onboarding/workspace?step=2"),
      );
      expect(res).not.toBeNull();
      expect(res?.status).toBe(307);
      const loc = new URL(res?.headers.get("location") ?? "");
      expect(loc.hostname).toBe("localhost");
      expect(loc.port).toBe("3000");
      expect(loc.pathname).toBe("/onboarding/workspace");
      expect(loc.search).toBe("?step=2");
    });

    it("does not 404 or mention demo mode on the loopback redirect", () => {
      const res = canonicalLoopbackRedirect(loopbackReq("/team"));
      expect(res?.status).not.toBe(404);
      const loc = res?.headers.get("location") ?? "";
      expect(loc.toLowerCase()).not.toContain("demo mode");
    });

    it("does NOT redirect a localhost request (no loop)", () => {
      // The helper returns null for the already-canonical host (so
      // `middleware()` falls through instead of redirect-looping)...
      expect(
        canonicalLoopbackRedirect(
          { nextUrl: new URL("http://localhost:3000/team") } as unknown as NextRequest,
        ),
      ).toBeNull();
      // ...and through the full middleware a localhost request is not
      // host-rewritten (session present, so no auth redirect either).
      const res = middleware(
        makeRequest("http://localhost:3000/team", { withSession: true }),
      );
      expect(res.headers.get("location")).toBeNull();
    });

    it("does NOT redirect a non-loopback host", () => {
      expect(
        canonicalLoopbackRedirect(
          { nextUrl: new URL("https://app.skycot.com/team") } as unknown as NextRequest,
        ),
      ).toBeNull();
    });

    it("keeps the session marker for a __Host-session cookie on localhost", () => {
      const req = new NextRequest(new URL("http://localhost:3000/team"));
      req.cookies.set("__Host-session", "test-jwt");
      const res = middleware(req);
      // Not redirected (canonical host + valid session) and the readable
      // companion marker is set so the client gates demo-mode OFF.
      expect(res.headers.get("location")).toBeNull();
      const marker = res.cookies.get(SESSION_MARKER_COOKIE);
      expect(marker?.value).toBe("1");
    });
  });
});
