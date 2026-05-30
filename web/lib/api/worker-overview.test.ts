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
