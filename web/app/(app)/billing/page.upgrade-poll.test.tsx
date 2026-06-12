/**
 * Regression pin — WS4 post-checkout tier auto-refresh.
 *
 * After Stripe Checkout succeeds it redirects the buyer to
 * `/billing?upgrade=ok`. The tier-flip webhook
 * (`POST /api/v1/billing/webhook`) is async and typically lands a beat
 * AFTER the redirect, so the tier the page first fetched is usually
 * still the pre-upgrade `free`. Before this fix `/billing` fired a
 * single `useQuery(["billing","usage"])` on mount and never reacted to
 * `?upgrade=ok`, so a just-paid customer sat on "Current tier: Free"
 * with no signal until they manually refreshed — the worst possible
 * moment for an "I paid and nothing changed" experience.
 *
 * The fix (web/app/(app)/billing/page.tsx): on `?upgrade=ok`, while the
 * fetched tier is still `free`, re-poll the SAME usage query (same
 * queryKey / fetchTenantUsage — no parallel fetch) with bounded backoff
 * until the tier flips, then stop. An honest "Processing your upgrade…"
 * note renders meanwhile (NO THEATER — never a fake "Pro active"); a
 * calm fallback renders if the bounded attempt cap is hit without a
 * flip. These tests pin that contract:
 *
 *  1. On `?upgrade=ok` with the usage query returning `free` then `pro`,
 *     the query is refetched MORE THAN ONCE (the page polls) and once it
 *     flips to `pro` the new plan renders and the processing note clears.
 *  2. On `?upgrade=ok` with the tier NEVER flipping, polling STOPS at
 *     the bounded attempt cap (no infinite refetch loop) and the calm
 *     "still processing" fallback renders — not a thrown error.
 *  3. WITHOUT `?upgrade=ok`, the page does its single existing fetch and
 *     does NOT poll (no behavior change for normal visits).
 */
import { act, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";
import type { UsageResponse } from "@/lib/api/usage";

// --- Router / search-params mock ----------------------------------------
let searchParams = new URLSearchParams();
vi.mock("next/navigation", () => ({
  useSearchParams: () => searchParams,
  useRouter: () => ({ replace: vi.fn(), push: vi.fn() }),
  usePathname: () => "/billing",
}));

// --- Usage fetcher SPY ---------------------------------------------------
// Partial mock: keep the real `usagePercent` (UsageMeter imports it) and
// the real `UsageResponse` type; only `fetchTenantUsage` is a spy so each
// test controls the tier sequence and asserts the refetch count.
const fetchTenantUsageSpy = vi.fn<() => Promise<UsageResponse>>();
vi.mock("@/lib/api/usage", async (orig) => ({
  ...(await orig<typeof import("@/lib/api/usage")>()),
  fetchTenantUsage: () => fetchTenantUsageSpy(),
}));

// SpendPanel + MeterPanel fire their own cost / meter / roster queries;
// stub them so this test stays focused on the usage-poll contract.
vi.mock("@/components/billing/SpendPanel", () => ({
  SpendPanel: () => null,
}));
vi.mock("@/components/billing/MeterPanel", () => ({
  MeterPanel: () => null,
}));

import BillingPage from "./page";

function usage(tier: UsageResponse["tier"], displayName: string): UsageResponse {
  return {
    tier,
    display_name: displayName,
    used: 0,
    limit: tier === "free" ? 100 : tier === "pro" ? 5000 : 0,
    remaining: tier === "free" ? 100 : null,
    resets_at: new Date(0).toISOString(),
  };
}

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

// Advance fake timers AND flush the promise microtask queue (query
// resolution) inside an act() so React state settles before assertions.
async function tick(ms: number) {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
}

const PROCESSING = /Processing your upgrade/i;
const STILL_PROCESSING = /Still processing\./i;

beforeEach(() => {
  vi.useFakeTimers();
  searchParams = new URLSearchParams();
  fetchTenantUsageSpy.mockReset();
});

afterEach(() => {
  vi.runOnlyPendingTimers();
  vi.useRealTimers();
  vi.clearAllMocks();
});

describe("/billing post-checkout tier auto-refresh", () => {
  it("polls the usage query on ?upgrade=ok and renders the new plan once the tier flips", async () => {
    searchParams = new URLSearchParams("upgrade=ok");
    fetchTenantUsageSpy
      .mockResolvedValueOnce(usage("free", "Free"))
      .mockResolvedValueOnce(usage("free", "Free"))
      .mockResolvedValue(usage("pro", "Pro plan"));

    renderBilling();
    await tick(0); // initial fetch resolves -> free

    expect(fetchTenantUsageSpy).toHaveBeenCalledTimes(1);
    expect(screen.getByText(PROCESSING)).toBeInTheDocument();

    await tick(1500); // poll #1 -> free
    expect(fetchTenantUsageSpy).toHaveBeenCalledTimes(2);
    expect(screen.getByText(PROCESSING)).toBeInTheDocument();

    await tick(2500); // poll #2 -> pro (flips)
    expect(fetchTenantUsageSpy).toHaveBeenCalledTimes(3);

    // New plan rendered; processing state gone.
    expect(screen.getByText("Pro plan")).toBeInTheDocument();
    expect(screen.queryByText(PROCESSING)).not.toBeInTheDocument();

    // No further polling after the flip.
    await tick(6000);
    await tick(6000);
    expect(fetchTenantUsageSpy).toHaveBeenCalledTimes(3);
  });

  it("stops polling at the bounded cap and shows a calm fallback when the tier never flips", async () => {
    searchParams = new URLSearchParams("upgrade=ok");
    fetchTenantUsageSpy.mockResolvedValue(usage("free", "Free"));

    renderBilling();
    await tick(0); // initial fetch -> free
    expect(fetchTenantUsageSpy).toHaveBeenCalledTimes(1);

    // Drive all five bounded poll attempts (1500+2500+4000+6000+6000).
    await tick(1500);
    await tick(2500);
    await tick(4000);
    await tick(6000);
    await tick(6000);

    // 1 initial + 5 polls, then it STOPS — no infinite loop.
    expect(fetchTenantUsageSpy).toHaveBeenCalledTimes(6);

    // Advancing further must not fire any more refetches.
    await tick(6000);
    await tick(6000);
    expect(fetchTenantUsageSpy).toHaveBeenCalledTimes(6);

    // Calm fallback, not an error.
    expect(screen.getByText(STILL_PROCESSING)).toBeInTheDocument();
    expect(screen.queryByText(PROCESSING)).not.toBeInTheDocument();
  });

  it("does its single existing fetch and does NOT poll without ?upgrade=ok", async () => {
    searchParams = new URLSearchParams(); // normal visit
    fetchTenantUsageSpy.mockResolvedValue(usage("free", "Free"));

    renderBilling();
    await tick(0);
    expect(fetchTenantUsageSpy).toHaveBeenCalledTimes(1);
    expect(screen.queryByText(PROCESSING)).not.toBeInTheDocument();
    expect(screen.queryByText(STILL_PROCESSING)).not.toBeInTheDocument();

    // No polling timers should fire.
    await tick(6000);
    await tick(6000);
    expect(fetchTenantUsageSpy).toHaveBeenCalledTimes(1);
  });
});
