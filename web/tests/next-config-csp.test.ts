import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { NextRequest } from "next/server";
import { middleware } from "../middleware";

/**
 * Server-free regression pin for the CSP `connect-src` directive baked
 * into every page response.
 *
 * History (do not regress): this pin was authored on 2026-05-29 (#2669)
 * when the CSP was a STATIC header in `next.config.ts` `headers()`, and it
 * called `config.headers()` directly. On 2026-06-08 (`f4f8a04e9`) the CSP
 * was deliberately moved OUT of `next.config.ts` and INTO `web/middleware.ts`
 * so it can carry a fresh per-request nonce and drop `script-src
 * 'unsafe-inline'` (`'nonce-…' + 'strict-dynamic'` instead). `next.config.ts`
 * now explicitly forbids re-adding a static CSP key — two CSP response
 * headers would both be enforced and the static (nonce-less) one would block
 * the framework scripts. So the directive this pin guards now lives in the
 * middleware response, and the pin sources it from there: it calls
 * `middleware()` with a synthetic request and reads the CSP off the response,
 * with no running server.
 *
 * Companion to the live e2e canary `tests/e2e/gateway-port-canary.spec.ts`:
 * that one needs a running server and reads the CSP off a real HTTP response,
 * this one calls `middleware()` directly so it runs in `pnpm test` / CI with
 * no gateway.
 *
 * The load-bearing assertion is the `ws(s):` origin. CSP `connect-src`
 * governs WebSocket handshakes too and treats `http:` and `ws:` as
 * distinct schemes, so the live approvals inbox
 * (`web/lib/hooks/use-approvals-live.ts`, which connects to
 * `${gateway}/inbox/ws`) is silently blocked unless the ws origin is
 * whitelisted. See brief `fix-approvals-live-ws-blocked-by-csp-connect-src`.
 */

const CANONICAL_LOCAL_GATEWAY_URL = "http://127.0.0.1:7400";

const ORIGINAL_NODE_ENV = process.env.NODE_ENV;
const ORIGINAL_BYPASS = process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
const ORIGINAL_DEMO_MODE = process.env.NEXT_PUBLIC_ARGO_DEMO_MODE;

function gatewayUrl(): string {
  return (
    process.env.NEXT_PUBLIC_GATEWAY_URL ??
    process.env.NEXT_PUBLIC_API_URL ??
    CANONICAL_LOCAL_GATEWAY_URL
  ).replace(/\/$/, "");
}

/**
 * Run the middleware against a synthetic request and pull the CSP
 * `connect-src` directive off the response header. The CSP is attached on
 * every middleware branch (pass-through and redirect alike), so the path is
 * immaterial — `/` mirrors the e2e canary's first-response check.
 */
function connectSrcDirective(): string {
  const req = new NextRequest(new URL("http://localhost:3000/"));
  const res = middleware(req);
  const cspHeader = res.headers.get("Content-Security-Policy");
  expect(cspHeader, "CSP header must be present").toBeTruthy();
  const directive = cspHeader!
    .split(";")
    .map((d) => d.trim())
    .find((d) => d.startsWith("connect-src "));
  expect(directive, "connect-src directive must be present").toBeDefined();
  return directive!;
}

describe("CSP connect-src", () => {
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
    Object.defineProperty(process.env, "NODE_ENV", {
      configurable: true,
      enumerable: true,
      writable: true,
      value: ORIGINAL_NODE_ENV,
    });
    if (ORIGINAL_BYPASS === undefined)
      delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
    else process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = ORIGINAL_BYPASS;
    if (ORIGINAL_DEMO_MODE === undefined)
      delete process.env.NEXT_PUBLIC_ARGO_DEMO_MODE;
    else process.env.NEXT_PUBLIC_ARGO_DEMO_MODE = ORIGINAL_DEMO_MODE;
  });

  it("whitelists the http(s) gateway origin (fetch)", () => {
    const directive = connectSrcDirective();
    expect(directive).toContain(gatewayUrl());
  });

  it("whitelists the ws(s) gateway origin so the live approvals WS handshake is allowed", () => {
    const directive = connectSrcDirective();
    const wsOrigin = gatewayUrl().replace(/^http(s?):/, "ws$1:");
    expect(directive).toContain(wsOrigin);
  });

  it("pins the production wss origin", () => {
    const directive = connectSrcDirective();
    expect(directive).toContain("wss://api.skycot.com");
  });

  it("never re-introduces the legacy 8787 gateway port", () => {
    const directive = connectSrcDirective();
    expect(directive).not.toContain("127.0.0.1:8787");
  });
});
