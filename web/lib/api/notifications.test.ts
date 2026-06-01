import { afterEach, describe, expect, it, vi } from "vitest";

// Control apiGet (the gateway call) and isDemoLive (recording-mode flag) so we
// can pin the NO-THEATER gate: in demo-live, a failed fetch must NEVER fall
// back to mock notifications — it surfaces the honest error instead.
const clientMock = vi.hoisted(() => ({ apiGet: vi.fn(), apiPost: vi.fn() }));
vi.mock("@/lib/api/client", () => clientMock);

const demoMock = vi.hoisted(() => ({ isDemoLive: vi.fn(() => false) }));
vi.mock("@/lib/skycot/demo-mode", () => demoMock);

import { fetchNotifications } from "./notifications";
import { ApiError } from "./types";
import { SAMPLE_NOTIFICATIONS } from "@/lib/sample-notifications";

afterEach(() => {
  vi.clearAllMocks();
  demoMock.isDemoLive.mockReturnValue(false);
});

describe("fetchNotifications — NO-THEATER demo-live gate", () => {
  it("returns the real gateway items on success (any mode)", async () => {
    const real = [{ id: "nt_real", title: "real seeded notification" }];
    clientMock.apiGet.mockResolvedValueOnce({ items: real, has_more: false, cursor: null });
    await expect(fetchNotifications()).resolves.toEqual(real);
  });

  it("falls back to mock on a 404 OUTSIDE demo-live (dev convenience)", async () => {
    demoMock.isDemoLive.mockReturnValue(false);
    clientMock.apiGet.mockRejectedValueOnce(new ApiError(404, "not found", "not_found"));
    await expect(fetchNotifications()).resolves.toEqual(SAMPLE_NOTIFICATIONS);
  });

  it("does NOT mock in demo-live — propagates the error (no fabricated rows on camera)", async () => {
    demoMock.isDemoLive.mockReturnValue(true);
    clientMock.apiGet.mockRejectedValueOnce(new ApiError(404, "not found", "not_found"));
    await expect(fetchNotifications()).rejects.toThrow();
  });

  it("does NOT mock in demo-live on a network error either", async () => {
    demoMock.isDemoLive.mockReturnValue(true);
    clientMock.apiGet.mockRejectedValueOnce(new TypeError("Failed to fetch"));
    await expect(fetchNotifications()).rejects.toThrow();
  });

  it("propagates non-404 ApiErrors regardless of mode", async () => {
    demoMock.isDemoLive.mockReturnValue(false);
    clientMock.apiGet.mockRejectedValueOnce(new ApiError(503, "down", "unavailable"));
    await expect(fetchNotifications()).rejects.toThrow();
  });
});
