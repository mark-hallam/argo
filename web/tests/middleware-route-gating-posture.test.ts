import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { NextRequest } from "next/server";
import { middleware } from "../middleware";

/**
 * web-route-gating-posture-logged-out (shakedown F4 + F5).
 *
 * The 2026-06-13 E2E shakedown found an INCONSISTENT logged-out posture
 * while the SPA ran with the dev auth-bypass on: `/` and a subset of app
 * routes (`/team`, `/audit`, `/approvals`, `/genesis`) rendered the app
 * shell as "Guest", while another subset (`/skills`, `/billing`, …)
 * redirected to `/auth/signin`. The brief's one rule is:
 *
 *   logged-out + no demo session  -> every app route redirects to
 *                                    `/auth/signin`, and bare `/` serves
 *                                    the marketing landing;
 *   demo session / real session   -> the full app surface opens uniformly.
 *
 * Dev auth-bypass intentionally keeps the open lane (so the operator can
 * walk the app locally), but the PRODUCTION posture is the one that ships
 * on the public flip, so it is what these tests pin. They exercise
 * `middleware()` with `NODE_ENV === "production"` and the bypass/demo env
 * vars unset, asserting the consistent shape across exactly the routes the
 * shakedown named — so a future allowlist edit that re-opens one of them
 * fails here instead of in front of a stranger.
 */

const ORIGINAL_NODE_ENV = process.env.NODE_ENV;
const ORIGINAL_BYPASS = process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
const ORIGINAL_DEMO_MODE = process.env.NEXT_PUBLIC_ARGO_DEMO_MODE;

function setNodeEnv(value: string) {
  Object.defineProperty(process.env, "NODE_ENV", {
    configurable: true,
    enumerable: true,
    writable: true,
    value,
  });
}

function makeRequest(
  path: string,
  { withSession }: { withSession?: boolean } = {},
) {
  const req = new NextRequest(new URL(path, "http://localhost:3000"));
  if (withSession) {
    req.cookies.set("__Host-session", "test-session-cookie");
  }
  return req;
}

function restoreEnv() {
  setNodeEnv(ORIGINAL_NODE_ENV ?? "test");
  if (ORIGINAL_BYPASS === undefined)
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
  else process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = ORIGINAL_BYPASS;
  if (ORIGINAL_DEMO_MODE === undefined)
    delete process.env.NEXT_PUBLIC_ARGO_DEMO_MODE;
  else process.env.NEXT_PUBLIC_ARGO_DEMO_MODE = ORIGINAL_DEMO_MODE;
}

// The full set of app routes the shakedown walked logged-out. F5 named the
// two halves explicitly; pinning both halves together is the whole point —
// the bug was that they behaved differently.
const F5_RENDERED_AS_GUEST = ["/team", "/audit", "/approvals", "/genesis"];
const F5_REDIRECTED_TO_SIGNIN = [
  "/skills",
  "/billing",
  "/settings",
  "/workflows",
  "/pitch",
  "/coach",
  "/cohorts",
  "/marketing",
  "/gtm",
];
const ALL_APP_ROUTES = [...F5_RENDERED_AS_GUEST, ...F5_REDIRECTED_TO_SIGNIN];

describe("production logged-out route-gating posture (shakedown F4 + F5)", () => {
  beforeEach(() => {
    setNodeEnv("production");
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
    delete process.env.NEXT_PUBLIC_ARGO_DEMO_MODE;
  });

  afterEach(() => {
    restoreEnv();
  });

  // F4: a stranger at the root must land on marketing, not an empty app shell.
  it("redirects logged-out `/` to the marketing landing", () => {
    const res = middleware(makeRequest("/"));
    expect(res.status).toBe(307);
    expect(res.headers.get("location")).toBe("http://localhost:3000/landing");
  });

  // F5: EVERY app route behaves the same way — redirect to signin with the
  // original path preserved in returnTo. No half-open surface.
  for (const path of ALL_APP_ROUTES) {
    it(`redirects logged-out ${path} to /auth/signin with returnTo`, () => {
      const res = middleware(makeRequest(path));
      expect(res.status).toBe(307);
      expect(res.headers.get("location")).toBe(
        `http://localhost:3000/auth/signin?returnTo=${encodeURIComponent(path)}`,
      );
    });
  }

  // The bypass-on observation in the shakedown was that these four leaked the
  // app shell as Guest. In production they must not — pinned as a group so a
  // regression on any one of them is caught.
  it("never renders the F5 'Guest' routes as a pass-through when logged out", () => {
    for (const path of F5_RENDERED_AS_GUEST) {
      const res = middleware(makeRequest(path));
      // A pass-through (NextResponse.next) carries no Location header; a gate does.
      expect(res.headers.get("location")).not.toBeNull();
    }
  });
});

describe("demo / real session opens the app surface uniformly (production)", () => {
  beforeEach(() => {
    setNodeEnv("production");
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
    delete process.env.NEXT_PUBLIC_ARGO_DEMO_MODE;
  });

  afterEach(() => {
    restoreEnv();
  });

  // A real (or minted demo) session cookie opens the same routes that bounce
  // a logged-out visitor — uniformly, including bare `/`.
  for (const path of ["/", ...ALL_APP_ROUTES]) {
    it(`lets a session through to ${path} without a signin redirect`, () => {
      const res = middleware(makeRequest(path, { withSession: true }));
      expect(res.headers.get("location")).toBeNull();
    });
  }
});
