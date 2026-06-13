/**
 * Regression pin — shakedown re-test N2 (`/billing` meter de-dup).
 *
 * The 2026-06-13 shakedown re-test found `/billing` rendering the
 * period-to-date metered-action count TWICE: once from `UsageMeter`
 * ("Actions this month 0/100", the plan cap) and once from `MeterPanel`
 * ("Actions this month 0/Unlimited included", the bundled allowance).
 * Two "Actions this month" rows reading the SAME `used_actions` source
 * is confusing duplication, and a bare 0 next to nonzero provider spend
 * read as broken.
 *
 * The fix: `UsageMeter` is now a plan-identity header only (tier name +
 * reset date) and the canonical "Actions this month" meter lives ONLY in
 * `MeterPanel`, with a unit label so a 0 count next to nonzero spend is
 * self-explanatory. This test renders the REAL `UsageMeter` + REAL
 * `MeterPanel` together (only `SpendPanel` and the fetchers are stubbed)
 * and pins that EXACTLY ONE "Actions this month" row renders.
 */
import { render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { UsageResponse } from "@/lib/api/usage";
import type { MeterResponse } from "@/lib/api/meter";

vi.mock("next/navigation", () => ({
  useSearchParams: () => new URLSearchParams(),
  useRouter: () => ({ replace: vi.fn(), push: vi.fn() }),
  usePathname: () => "/billing",
}));

const fetchTenantUsageSpy = vi.fn<() => Promise<UsageResponse>>();
vi.mock("@/lib/api/usage", async (orig) => ({
  ...(await orig<typeof import("@/lib/api/usage")>()),
  fetchTenantUsage: () => fetchTenantUsageSpy(),
}));

const fetchBillingMeterSpy = vi.fn<() => Promise<MeterResponse>>();
vi.mock("@/lib/api/meter", async (orig) => ({
  ...(await orig<typeof import("@/lib/api/meter")>()),
  fetchBillingMeter: () => fetchBillingMeterSpy(),
}));

// SpendPanel fires its own cost / roster queries; stub it so this test
// stays focused on the meter de-dup contract. UsageMeter + MeterPanel are
// the REAL components under test.
vi.mock("@/components/billing/SpendPanel", () => ({
  SpendPanel: () => null,
}));

import BillingPage from "./page";

function renderBilling() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: Infinity } },
  });
  return render(
    <QueryClientProvider client={client}>
      <BillingPage />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  fetchTenantUsageSpy.mockReset();
  fetchBillingMeterSpy.mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("/billing meter de-dup (shakedown re-test N2)", () => {
  it("renders EXACTLY ONE 'Actions this month' row across UsageMeter + MeterPanel", async () => {
    // The reported case: 0 metered actions while spend accrues.
    fetchTenantUsageSpy.mockResolvedValue({
      tier: "free",
      display_name: "Free",
      used: 0,
      limit: 100,
      remaining: 100,
      resets_at: "2026-07-01T00:00:00Z",
    });
    fetchBillingMeterSpy.mockResolvedValue({
      tier: "free",
      display_name: "Free",
      included_actions: 100,
      used_actions: 0,
      overage_units: 0,
      overage_multiplier: 1,
      estimated_overage_usd: 0,
      cap_usd: null,
      cap_is_default: true,
    });

    renderBilling();

    // Wait for the canonical meter (MeterPanel) to settle.
    await waitFor(() =>
      expect(screen.getByTestId("meter-panel")).toBeInTheDocument(),
    );

    // Plan-identity header is present, but it no longer carries its own
    // metered "Actions this month" row.
    expect(screen.getByText("Current plan")).toBeInTheDocument();

    // The de-dup contract: one and only one "Actions this month" label.
    expect(screen.getAllByText("Actions this month")).toHaveLength(1);

    // And the unit is labelled so 0-with-spend is self-explanatory.
    expect(screen.getByTestId("meter-unit-note")).toBeInTheDocument();
  });
});
