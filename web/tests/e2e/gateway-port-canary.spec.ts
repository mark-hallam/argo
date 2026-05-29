import { expect, test } from "@playwright/test";

/**
 * Canary spec for the canonical-gateway-port fix
 * (`fix-csp-connect-src-gateway-port-drift`).
 *
 * Asserts that the Content-Security-Policy `connect-src` directive
 * emitted on the first page response whitelists the same gateway URL
 * the SPA's fetch hooks read from `NEXT_PUBLIC_GATEWAY_URL` (or the
 * canonical localhost fallback when unset). If the two ever drift
 * apart the browser will silently block every gateway fetch with
 * only a console line as evidence — a P1 deploy hazard.
 *
 * The legacy drift value `http://127.0.0.1:8787` shows up in older
 * BUILD_STATUS audit notes; the regression assertion keeps it
 * blacklisted in perpetuity.
 */

const CANONICAL_LOCAL_GATEWAY_URL = "http://127.0.0.1:7400";

function expectedConnectSrc(): string {
  return (
    process.env.NEXT_PUBLIC_GATEWAY_URL ??
    process.env.NEXT_PUBLIC_API_URL ??
    CANONICAL_LOCAL_GATEWAY_URL
  );
}

test("CSP connect-src whitelists the active gateway URL and never the legacy 8787 port", async ({
  request,
}) => {
  const res = await request.get("/");
  expect(res.status()).toBeLessThan(500);
  const csp = res.headers()["content-security-policy"] ?? "";
  expect(csp).not.toEqual("");
  expect(csp).toContain(`connect-src`);
  expect(csp).toContain(expectedConnectSrc());
  // CSP connect-src governs WebSocket handshakes too, and ws: is a
  // distinct scheme from http:. The live approvals inbox connects to
  // `${gateway}/inbox/ws`, so the ws-scheme origin must be whitelisted
  // or the browser silently refuses the handshake.
  expect(csp).toContain(expectedConnectSrc().replace(/^http(s?):/, "ws$1:"));
  expect(csp).not.toContain("http://127.0.0.1:8787");
});
