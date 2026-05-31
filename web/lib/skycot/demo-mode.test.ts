/**
 * Regression suite for the recording-mode readiness warning
 * (brief `fix-demo-recording-disable-auth-bypass-prep`).
 *
 * Pins the contract that `recordingModeReadinessWarning()` returns:
 *  - `null` for every safe combination (offline UI dev, real signed-in
 *    production, recording-day with bypass off)
 *  - a single warning string ONLY when both `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`
 *    AND `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` are set under a non-
 *    production NODE_ENV (the offline UI iteration default that is
 *    structurally incapable of exercising the live gateway)
 *
 * The warning is the only signal the operator gets that the SPA's
 * 28 `lib/api/*` fetchers are about to short-circuit to seed fixtures
 * before any gateway request leaves the browser. Losing this regression
 * means a pre-record session can run end-to-end without ever touching
 * the gateway and the operator has no way to know.
 */
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import {
  isDemoMode,
  isDemoLive,
  isGatewayAuthBypassed,
  recordingModeReadinessWarning,
} from "@/lib/skycot/demo-mode";

const ORIGINAL_ENV = { ...process.env };

beforeEach(() => {
  process.env = { ...ORIGINAL_ENV, NODE_ENV: "development" };
  delete process.env.NEXT_PUBLIC_ENV;
  delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE;
  delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE;
  delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
});

afterEach(() => {
  process.env = ORIGINAL_ENV;
});

describe("flag readers", () => {
  it("isDemoMode reads NEXT_PUBLIC_SKYCOT_DEMO_MODE", () => {
    expect(isDemoMode()).toBe(false);
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE = "1";
    expect(isDemoMode()).toBe(true);
  });

  it("isDemoLive reads NEXT_PUBLIC_SKYCOT_DEMO_LIVE", () => {
    expect(isDemoLive()).toBe(false);
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    expect(isDemoLive()).toBe(true);
  });

  it("isGatewayAuthBypassed reads NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS under non-prod", () => {
    expect(isGatewayAuthBypassed()).toBe(false);
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    expect(isGatewayAuthBypassed()).toBe(true);
  });

  it("isGatewayAuthBypassed returns false in production even with the env var set", () => {
    process.env = {
      ...ORIGINAL_ENV,
      NODE_ENV: "production",
      NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS: "1",
    };
    expect(isGatewayAuthBypassed()).toBe(false);
  });
});

describe("recordingModeReadinessWarning", () => {
  it("returns null when nothing is set (signed-in production happy path)", () => {
    expect(recordingModeReadinessWarning()).toBeNull();
  });

  it("returns null when only demo-mode is set (signed-in demo workspace)", () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE = "1";
    expect(recordingModeReadinessWarning()).toBeNull();
  });

  it("returns null when only auth-bypass is set (offline dev, no demo banner)", () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    expect(recordingModeReadinessWarning()).toBeNull();
  });

  it("returns a warning string when demo-mode AND auth-bypass are set (offline-dev, no demo-live)", () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE = "1";
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    const warning = recordingModeReadinessWarning();
    expect(warning).not.toBeNull();
    expect(warning).toContain("DEMO-RECORDING WARNING");
    expect(warning).toContain("NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=0");
  });

  it("returns null under the demo-live recording profile (the live fetchers DO hit the gateway)", () => {
    // demo-live IS the profile the warning tells the operator to reach;
    // firing it there is stale AND shows as a camera-visible "1 Issue"
    // chip in the Next.js dev overlay. See
    // fix-worker-personality-and-dev-overlay-issue.
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE = "1";
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    expect(recordingModeReadinessWarning()).toBeNull();
  });

  it("returns null in production even with both flags set (bypass guard wins)", () => {
    process.env = {
      ...ORIGINAL_ENV,
      NODE_ENV: "production",
      NEXT_PUBLIC_SKYCOT_DEMO_MODE: "1",
      NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS: "1",
    };
    expect(recordingModeReadinessWarning()).toBeNull();
  });
});
