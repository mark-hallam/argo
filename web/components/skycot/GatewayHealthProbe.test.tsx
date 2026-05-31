import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { GatewayHealthProbe } from "./GatewayHealthProbe";

/**
 * Regression coverage for the false "Gateway unreachable" banner.
 *
 * The bug: the probe did a DIRECT cross-origin fetch to
 * `http://127.0.0.1:7400/health`, which the browser refuses (the Rust
 * gateway sends no `Access-Control-Allow-Origin`), so the amber banner
 * rendered even though the gateway was healthy and the real data path
 * (the same-origin Next proxy) worked fine.
 *
 * The fix: probe the SAME-ORIGIN `/health` path the Next rewrite
 * proxies through to the gateway — the exact mechanism the real data
 * path uses. These tests pin (a) the probe never crosses origins, and
 * (b) the banner is gone on a healthy proxy but still shows on a real
 * failure.
 */
describe("GatewayHealthProbe", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    // Ensure the demo-mode / production short-circuits are inactive so
    // the probe actually runs its fetch.
    delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE;
    fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("probes the same-origin /health path, never the cross-origin gateway URL", async () => {
    fetchMock.mockResolvedValue(new Response("{}", { status: 200 }));
    render(<GatewayHealthProbe />);

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    const calledUrl = String(fetchMock.mock.calls[0]?.[0]);
    expect(calledUrl).toBe("/health");
    // The historical bug: probing the absolute :7400 origin directly.
    expect(calledUrl).not.toContain("7400");
    expect(calledUrl).not.toContain("http");
  });

  it("renders no banner when the proxied /health probe succeeds", async () => {
    fetchMock.mockResolvedValue(new Response("{}", { status: 200 }));
    render(<GatewayHealthProbe />);

    await waitFor(() => expect(fetchMock).toHaveBeenCalled());
    // Give React a tick to flush the resolved-state render.
    await waitFor(() =>
      expect(
        screen.queryByTestId("gateway-health-probe-banner"),
      ).not.toBeNull(),
    ).catch(() => {});
    expect(screen.queryByTestId("gateway-health-probe-banner")).toBeNull();
  });

  it("still surfaces the banner when the proxied probe genuinely fails", async () => {
    fetchMock.mockRejectedValue(new Error("network down"));
    render(<GatewayHealthProbe />);

    const banner = await screen.findByTestId("gateway-health-probe-banner");
    expect(banner).not.toBeNull();
    expect(banner.textContent).toContain("Gateway unreachable");
    // The copy reflects the real data path (the proxy), not a direct fetch.
    expect(banner.textContent).toContain("same-origin /health proxy");
  });
});
