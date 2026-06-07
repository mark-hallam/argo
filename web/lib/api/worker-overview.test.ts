/**
 * Tests for `fetchWorkerOverview`'s gateway-shape normalisation —
 * specifically `mapGatewayDuty`, which turns the live gateway duty
 * shape (`schedule: { kind, spec }`) into the SPA `WorkerDuty` contract
 * (`schedule: string`). Before this mapping the Responsibilities /
 * Schedules card rendered `[object Object]` for every duty chip.
 *
 * Drives the GATEWAY branch (not the auth-bypass mock branch) by setting
 * `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` — the recording-mode opt-out — and
 * mocking `apiGet` so no real gateway is needed. Mirrors the harness in
 * `demo-live-mode.test.ts`.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const apiGetMock = vi.fn();
vi.mock("@/lib/api/client", () => ({
  apiGet: (...args: unknown[]) => apiGetMock(...args),
}));

const ORIGINAL_ENV = { ...process.env };

beforeEach(() => {
  apiGetMock.mockReset();
  vi.resetModules();
  process.env = { ...ORIGINAL_ENV, NODE_ENV: "development" };
  delete process.env.NEXT_PUBLIC_ENV;
  delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE;
  process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
  // Recording mode → fetchWorkerOverview hits the (mocked) gateway.
  process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
});

afterEach(() => {
  process.env = ORIGINAL_ENV;
});

const WORKER_ID = "11111111-1111-1111-1111-111111111111";

describe("fetchWorkerOverview — gateway duty shape mapping", () => {
  it("maps {kind:'cron', spec} schedules to a readable cadence string", async () => {
    apiGetMock.mockResolvedValueOnce({
      charter: { text: "Lists and re-prices items.", version: 1 },
      duties: [
        {
          id: "d1",
          label: "Re-price stale listings against competitor data",
          schedule: { kind: "cron", spec: "0 30 6 * * *" },
          description: "Re-price stale listings against competitor data",
          enabled: true,
          next_fire_at: "2026-05-31T06:30:00Z",
        },
        {
          id: "d2",
          label: "List new inventory with photos + tags",
          schedule: { kind: "cron", spec: "0 */15 * * * *" },
          enabled: true,
        },
      ],
      kpis: [],
      recent_activity: [],
    });

    const { fetchWorkerOverview } = await import("@/lib/api/worker-overview");
    const { overview, source } = await fetchWorkerOverview(WORKER_ID);

    expect(source).toBe("gateway");
    expect(overview.duties).toHaveLength(2);
    // Daily 06:30 from a 6-field cron.
    expect(overview.duties[0].schedule).toBe("daily 06:30");
    expect(overview.duties[0].label).toBe(
      "Re-price stale listings against competitor data",
    );
    expect(overview.duties[0].enabled).toBe(true);
    // every-N-minutes cadence.
    expect(overview.duties[1].schedule).toBe("every 15 min");
    // No `[object Object]` leaked into any chip.
    for (const d of overview.duties) {
      expect(d.schedule).not.toContain("[object Object]");
    }
  });

  it("renders weekday + event cadences readably", async () => {
    apiGetMock.mockResolvedValueOnce({
      charter: "",
      duties: [
        {
          id: "d3",
          label: "Book first-round interviews",
          schedule: { kind: "cron", spec: "0 0 9 * * 1-5" },
          enabled: true,
        },
        {
          id: "d4",
          label: "Reply as messages arrive",
          schedule: { kind: "event", spec: "message.received" },
          enabled: false,
        },
      ],
      kpis: [],
      recent_activity: [],
    });

    const { fetchWorkerOverview } = await import("@/lib/api/worker-overview");
    const { overview } = await fetchWorkerOverview(WORKER_ID);

    expect(overview.duties[0].schedule).toBe("weekdays 09:00");
    expect(overview.duties[1].schedule).toBe("on: message.received");
    expect(overview.duties[1].enabled).toBe(false);
  });

  it("tolerates the already-string schedule shape from mocks", async () => {
    apiGetMock.mockResolvedValueOnce({
      charter: "",
      duties: [
        { id: "d5", label: "Legacy duty", schedule: "weekdays 09:00–17:00", enabled: true },
      ],
      kpis: [],
      recent_activity: [],
    });

    const { fetchWorkerOverview } = await import("@/lib/api/worker-overview");
    const { overview } = await fetchWorkerOverview(WORKER_ID);

    expect(overview.duties[0].schedule).toBe("weekdays 09:00–17:00");
  });

  it("never crashes on an empty duties list", async () => {
    apiGetMock.mockResolvedValueOnce({
      charter: "",
      duties: [],
      kpis: [],
      recent_activity: [],
    });

    const { fetchWorkerOverview } = await import("@/lib/api/worker-overview");
    const { overview } = await fetchWorkerOverview(WORKER_ID);

    expect(overview.duties).toEqual([]);
  });
});

describe("fetchWorkerOverview — recent activity normalisation", () => {
  // The gateway returns activity rows as { id, verb, status, timestamp }
  // with NO `href` (and `timestamp`, not `at`). RecentActivityCard renders
  // `<Link href={row.href}>`, and Next's `<Link href={undefined}>` THROWS —
  // crashing the entire worker-overview tab (the "500 — couldn't load this
  // worker" page). These pin that the mapper builds a real href + `at` and
  // never lets `href` reach the component as `undefined`.
  it("builds the audit drill-in href + `at` from a gateway row that has neither", async () => {
    apiGetMock.mockResolvedValueOnce({
      charter: "",
      duties: [],
      kpis: [],
      recent_activity: [
        {
          id: "019e9b1b-82d4-7e72-a3bb-6ed1d3ee48f1",
          verb: "trust_grade_changed",
          status: "succeeded",
          timestamp: "2026-06-06T04:27:48.972878+00:00",
        },
      ],
    });

    const { fetchWorkerOverview } = await import("@/lib/api/worker-overview");
    const { overview } = await fetchWorkerOverview(WORKER_ID);

    expect(overview.recentActivity).toHaveLength(1);
    const row = overview.recentActivity[0];
    // Never `undefined` — that is exactly what crashes <Link>.
    expect(typeof row.href).toBe("string");
    // Dashes stripped to match the `/audit/action/<32-hex>` route.
    expect(row.href).toBe(
      "/audit/action/019e9b1b82d47e72a3bb6ed1d3ee48f1",
    );
    expect(row.at).toBe("2026-06-06T04:27:48.972878+00:00");
    expect(row.verb).toBe("trust_grade_changed");
    expect(row.status).toBe("succeeded");
  });

  it("leaves href as an empty string (never undefined) when a row has no id", async () => {
    apiGetMock.mockResolvedValueOnce({
      charter: "",
      duties: [],
      kpis: [],
      recent_activity: [
        { verb: "did a thing", status: "ok", timestamp: "2026-06-06T00:00:00Z" },
      ],
    });

    const { fetchWorkerOverview } = await import("@/lib/api/worker-overview");
    const { overview } = await fetchWorkerOverview(WORKER_ID);

    // Empty string is falsy → the card renders a non-link row; crucially it
    // is a string, not `undefined`, so <Link> can never receive undefined.
    expect(overview.recentActivity[0].href).toBe("");
    expect(typeof overview.recentActivity[0].href).toBe("string");
  });
});
