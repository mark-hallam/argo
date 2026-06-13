import { render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import type { MeterResponse } from "@/lib/api/meter";

// Spy the fetcher; keep the real `meterUsagePercent` the component uses.
const fetchBillingMeterSpy = vi.fn<() => Promise<MeterResponse>>();
vi.mock("@/lib/api/meter", async (orig) => ({
  ...(await orig<typeof import("@/lib/api/meter")>()),
  fetchBillingMeter: () => fetchBillingMeterSpy(),
}));

import { MeterPanel } from "./MeterPanel";

function meter(overrides: Partial<MeterResponse> = {}): MeterResponse {
  return {
    tier: "pro",
    display_name: "Pro plan",
    included_actions: 1500,
    used_actions: 1000,
    overage_units: 0,
    overage_multiplier: 1.4,
    estimated_overage_usd: 0,
    cap_usd: 200,
    cap_is_default: true,
    ...overrides,
  };
}

function renderPanel() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: Infinity } },
  });
  return render(
    <QueryClientProvider client={client}>
      <MeterPanel />
    </QueryClientProvider>,
  );
}

describe("MeterPanel", () => {
  it("renders real included/used counts and an honest no-overage state", async () => {
    fetchBillingMeterSpy.mockResolvedValue(meter());
    renderPanel();

    await waitFor(() =>
      expect(screen.getByTestId("meter-panel")).toBeInTheDocument(),
    );
    expect(screen.getByText(/1,000 \/ 1,500 included/)).toBeInTheDocument();
    const bar = screen.getByRole("progressbar", {
      name: /Included action allowance used/,
    });
    // 1000 / 1500 ≈ 66.67%
    expect(Number(bar.getAttribute("aria-valuenow"))).toBeCloseTo(66.67, 0);
    expect(screen.getByText("None")).toBeInTheDocument(); // overage
    expect(screen.getByText(/\$200\.00 \(default\)/)).toBeInTheDocument();
  });

  it("labels the meter unit so a 0 count next to nonzero spend reads as honest, not broken", async () => {
    // The core shakedown re-test N2 case: a tenant whose metered-action
    // counter is 0 while provider spend accrues. The note must make the
    // unit self-explanatory (billable metered actions ≠ provider spend).
    fetchBillingMeterSpy.mockResolvedValue(
      meter({ used_actions: 0, included_actions: 1500 }),
    );
    renderPanel();

    await waitFor(() =>
      expect(screen.getByTestId("meter-unit-note")).toBeInTheDocument(),
    );
    expect(screen.getByText(/0 \/ 1,500 included/)).toBeInTheDocument();
    expect(
      screen.getByText(/billable metered actions/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Provider spend is metered\s+separately/i),
    ).toBeInTheDocument();
  });

  it("surfaces overage units + a labelled dollar estimate", async () => {
    fetchBillingMeterSpy.mockResolvedValue(
      meter({
        used_actions: 2000,
        overage_units: 500,
        estimated_overage_usd: 0.7,
      }),
    );
    renderPanel();

    await waitFor(() =>
      expect(screen.getByText(/500 actions/)).toBeInTheDocument(),
    );
    expect(screen.getByText(/\$0\.7000/)).toBeInTheDocument();
    expect(screen.getByText("est.")).toBeInTheDocument();
  });

  it("settles to an honest error state, never an infinite spinner", async () => {
    fetchBillingMeterSpy.mockRejectedValue(new Error("gateway down"));
    renderPanel();

    // The component retries once (~1s backoff) before settling, so allow
    // headroom past the 1s waitFor default — the point is it DOES settle.
    await waitFor(
      () => expect(screen.getByTestId("meter-error")).toBeInTheDocument(),
      { timeout: 4000 },
    );
    expect(screen.queryByTestId("meter-loading")).not.toBeInTheDocument();
    expect(
      screen.getByText(/Usage meter is unavailable right now/),
    ).toBeInTheDocument();
  });

  it("renders no metered allowance for a zero-allowance tier", async () => {
    fetchBillingMeterSpy.mockResolvedValue(
      meter({
        tier: "enterprise",
        display_name: "Enterprise",
        included_actions: 0,
        used_actions: 4200,
        cap_usd: null,
      }),
    );
    renderPanel();

    await waitFor(() =>
      expect(
        screen.getByText(/No metered allowance on this plan/),
      ).toBeInTheDocument(),
    );
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
    expect(screen.getByText("No cap")).toBeInTheDocument();
  });
});
