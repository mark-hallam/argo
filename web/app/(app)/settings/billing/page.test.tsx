/**
 * Regression pin — soak final scorecard 2026-06-13
 * (`web-settings-tier-label-mismatch-with-billing`).
 *
 * The soak found `/settings/billing` rendering a hardcoded "Pro" plan
 * label while the canonical `/billing` page showed "Free" for the same
 * demo tenant. Two surfaces disagreed because Settings ignored the live
 * tier and `/billing` read it from `GET /api/v1/tenant/usage`.
 *
 * The fix: `/settings/billing` now reads the plan label from the SAME
 * source via `fetchTenantUsage()` → `display_name`. This test stubs the
 * canonical fetcher with a `free` tenant fixture and pins that BOTH the
 * settings panel and the billing `UsageMeter` render the SAME label
 * ("Free"), and that the stale hardcoded "Pro" is gone from Settings.
 */
import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { UsageResponse } from "@/lib/api/usage";

const fetchTenantUsageSpy = vi.fn<() => Promise<UsageResponse>>();
vi.mock("@/lib/api/usage", async (orig) => ({
  ...(await orig<typeof import("@/lib/api/usage")>()),
  fetchTenantUsage: () => fetchTenantUsageSpy(),
}));

import BillingSettingsPage from "./page";
import { UsageMeter } from "@/components/billing/UsageMeter";

const FREE_FIXTURE: UsageResponse = {
  tier: "free",
  display_name: "Free",
  used: 0,
  limit: 100,
  remaining: 100,
  resets_at: "2026-07-01T00:00:00Z",
};

beforeEach(() => {
  fetchTenantUsageSpy.mockReset();
  fetchTenantUsageSpy.mockResolvedValue(FREE_FIXTURE);
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("settings billing plan label", () => {
  it("reads the canonical tier label from fetchTenantUsage", async () => {
    render(<BillingSettingsPage />);
    const plan = await screen.findByTestId("settings-billing-plan");
    await waitFor(() => expect(plan).toHaveTextContent("Free"));
  });

  it("no longer hardcodes 'Pro' for a free tenant", async () => {
    render(<BillingSettingsPage />);
    await screen.findByTestId("settings-billing-plan");
    await waitFor(() =>
      expect(screen.getByTestId("settings-billing-plan")).toHaveTextContent("Free"),
    );
    expect(screen.queryByText("Pro")).toBeNull();
  });

  it("renders the SAME label that /billing's UsageMeter shows", async () => {
    // `/billing` (UsageMeter) and `/settings/billing` consume the SAME
    // `display_name` field — so a single fixture must drive both. Scope
    // each query to its own render so the shared "Free" text in the DOM
    // doesn't trip the (correct) duplicate-match — that duplication IS
    // the parity we're pinning.
    const settings = render(<BillingSettingsPage />);
    const settingsPlan = await settings.findByTestId("settings-billing-plan");
    await waitFor(() => expect(settingsPlan).toHaveTextContent("Free"));

    const meter = render(<UsageMeter usage={FREE_FIXTURE} />, {
      container: document.body.appendChild(document.createElement("div")),
    });
    expect(meter.getByText(FREE_FIXTURE.display_name)).toBeInTheDocument();
    expect(settingsPlan).toHaveTextContent(FREE_FIXTURE.display_name);
  });
});
