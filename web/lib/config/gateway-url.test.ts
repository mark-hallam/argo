/**
 * Smoke coverage for the canonical gateway-url module. Verifies the
 * three behaviours every consumer relies on: trailing-slash
 * normalisation, the canonical-local detection, and that the active
 * URL falls back to the documented dev port when no env override is
 * set at call time.
 */
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import {
  CANONICAL_LOCAL_GATEWAY_URL,
  gatewayHealthUrl,
  isCanonicalLocalGateway,
  normaliseGatewayUrl,
  resolveGatewayUrl,
} from "./gateway-url";

describe("gateway-url canonical module", () => {
  const originalGw = process.env.NEXT_PUBLIC_GATEWAY_URL;
  const originalApi = process.env.NEXT_PUBLIC_API_URL;

  beforeEach(() => {
    delete process.env.NEXT_PUBLIC_GATEWAY_URL;
    delete process.env.NEXT_PUBLIC_API_URL;
  });

  afterEach(() => {
    if (originalGw === undefined) delete process.env.NEXT_PUBLIC_GATEWAY_URL;
    else process.env.NEXT_PUBLIC_GATEWAY_URL = originalGw;
    if (originalApi === undefined) delete process.env.NEXT_PUBLIC_API_URL;
    else process.env.NEXT_PUBLIC_API_URL = originalApi;
  });

  it("exposes the documented localhost dev fallback", () => {
    expect(CANONICAL_LOCAL_GATEWAY_URL).toBe("http://127.0.0.1:7400");
  });

  it("normaliseGatewayUrl strips trailing slashes", () => {
    expect(normaliseGatewayUrl("http://127.0.0.1:7400/")).toBe(
      "http://127.0.0.1:7400",
    );
    expect(normaliseGatewayUrl("http://127.0.0.1:7400//")).toBe(
      "http://127.0.0.1:7400",
    );
    expect(normaliseGatewayUrl("http://127.0.0.1:7400")).toBe(
      "http://127.0.0.1:7400",
    );
  });

  it("resolveGatewayUrl falls back to canonical when no env is set", () => {
    expect(resolveGatewayUrl()).toBe(CANONICAL_LOCAL_GATEWAY_URL);
  });

  it("resolveGatewayUrl honours NEXT_PUBLIC_GATEWAY_URL", () => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "https://api.skycot.com";
    expect(resolveGatewayUrl()).toBe("https://api.skycot.com");
  });

  it("resolveGatewayUrl falls through to NEXT_PUBLIC_API_URL when NEXT_PUBLIC_GATEWAY_URL is unset", () => {
    process.env.NEXT_PUBLIC_API_URL = "https://api.skycot.com/";
    expect(resolveGatewayUrl()).toBe("https://api.skycot.com");
  });

  it("isCanonicalLocalGateway matches the documented port regardless of trailing slash", () => {
    expect(isCanonicalLocalGateway("http://127.0.0.1:7400")).toBe(true);
    expect(isCanonicalLocalGateway("http://127.0.0.1:7400/")).toBe(true);
    expect(isCanonicalLocalGateway("http://127.0.0.1:8787")).toBe(false);
    expect(isCanonicalLocalGateway("https://api.skycot.com")).toBe(false);
  });

  it("gatewayHealthUrl composes a `/health` suffix without double slash", () => {
    expect(gatewayHealthUrl("http://127.0.0.1:7400")).toBe(
      "http://127.0.0.1:7400/health",
    );
    expect(gatewayHealthUrl("http://127.0.0.1:7400/")).toBe(
      "http://127.0.0.1:7400/health",
    );
  });
});
