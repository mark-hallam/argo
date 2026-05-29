import { describe, expect, it } from "vitest";
import config from "../next.config";

/**
 * Server-free regression pin for the CSP `connect-src` directive baked
 * into every page response by `next.config.ts` `headers()`.
 *
 * Companion to the live e2e canary `tests/e2e/gateway-port-canary.spec.ts`:
 * that one needs a running server, this one calls `config.headers()`
 * directly so it runs in `pnpm test` / CI with no gateway.
 *
 * The load-bearing assertion is the `ws(s):` origin. CSP `connect-src`
 * governs WebSocket handshakes too and treats `http:` and `ws:` as
 * distinct schemes, so the live approvals inbox
 * (`web/lib/hooks/use-approvals-live.ts`, which connects to
 * `${gateway}/inbox/ws`) is silently blocked unless the ws origin is
 * whitelisted. See brief `fix-approvals-live-ws-blocked-by-csp-connect-src`.
 */

const CANONICAL_LOCAL_GATEWAY_URL = "http://127.0.0.1:7400";

function gatewayUrl(): string {
  return (
    process.env.NEXT_PUBLIC_GATEWAY_URL ??
    process.env.NEXT_PUBLIC_API_URL ??
    CANONICAL_LOCAL_GATEWAY_URL
  ).replace(/\/$/, "");
}

async function connectSrcDirective(): Promise<string> {
  if (typeof config.headers !== "function") {
    throw new Error("next.config headers() is not a function");
  }
  const groups = await config.headers();
  const cspHeader = groups
    .flatMap((g) => g.headers)
    .find((h) => h.key === "Content-Security-Policy");
  expect(cspHeader, "CSP header must be present").toBeDefined();
  const directive = cspHeader!.value
    .split(";")
    .map((d) => d.trim())
    .find((d) => d.startsWith("connect-src "));
  expect(directive, "connect-src directive must be present").toBeDefined();
  return directive!;
}

describe("CSP connect-src", () => {
  it("whitelists the http(s) gateway origin (fetch)", async () => {
    const directive = await connectSrcDirective();
    expect(directive).toContain(gatewayUrl());
  });

  it("whitelists the ws(s) gateway origin so the live approvals WS handshake is allowed", async () => {
    const directive = await connectSrcDirective();
    const wsOrigin = gatewayUrl().replace(/^http(s?):/, "ws$1:");
    expect(directive).toContain(wsOrigin);
  });

  it("pins the production wss origin", async () => {
    const directive = await connectSrcDirective();
    expect(directive).toContain("wss://api.skycot.com");
  });

  it("never re-introduces the legacy 8787 gateway port", async () => {
    const directive = await connectSrcDirective();
    expect(directive).not.toContain("127.0.0.1:8787");
  });
});
