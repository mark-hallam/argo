import { afterEach, describe, expect, it, vi } from "vitest";

// Control the gateway client + the real-session flag so we can pin the
// mock-leak class gate: a REAL signed-in user must NEVER be served a
// fabricated schedule on a 404 / malformed body — they get an honest error.
// Anonymous dev/demo keeps the deterministic mock so the wizard is testable
// with the gateway offline. The gateway happy path is unchanged for both.
const clientMock = vi.hoisted(() => ({
  apiPost: vi.fn(),
  apiPatch: vi.fn(),
  apiGet: vi.fn(),
}));
vi.mock("@/lib/api/client", () => clientMock);

const demoMock = vi.hoisted(() => ({ hasRealSession: vi.fn(() => false) }));
vi.mock("@/lib/skycot/demo-mode", () => demoMock);

import { classifyDuty, listDuties } from "./duties";
import { ApiError } from "./types";

const REQ = {
  workerId: "w1",
  input: "every monday at 9am",
  schedule: { kind: "cron", spec: "0 9 * * 1" },
};
const VALID_PROPOSAL = {
  kind: "recurring",
  label: "Weekly report",
  description: "Send the weekly status report",
  schedule: { kind: "cron", spec: "0 9 * * 1" },
};

afterEach(() => {
  vi.clearAllMocks();
  demoMock.hasRealSession.mockReturnValue(false);
});

describe("duties — real-session NO-THEATER gate (mock-leak class)", () => {
  it("happy path: a valid gateway proposal is returned unchanged (real session)", async () => {
    demoMock.hasRealSession.mockReturnValue(true);
    clientMock.apiPost.mockResolvedValueOnce(VALID_PROPOSAL);
    const r = await classifyDuty(REQ as never);
    expect(r.source).toBe("gateway");
    expect(r.proposal).toEqual(VALID_PROPOSAL);
  });

  it("anonymous: a 404 falls back to a mock proposal (dev convenience)", async () => {
    clientMock.apiPost.mockRejectedValueOnce(new ApiError(404, "nf", "not_found"));
    const r = await classifyDuty(REQ as never);
    expect(r.source).toBe("mock");
  });

  it("REAL user: a 404 surfaces an honest error, NOT a fabricated proposal", async () => {
    demoMock.hasRealSession.mockReturnValue(true);
    clientMock.apiPost.mockRejectedValueOnce(new ApiError(404, "nf", "not_found"));
    await expect(classifyDuty(REQ as never)).rejects.toBeInstanceOf(ApiError);
  });

  it("REAL user: a malformed gateway body surfaces an error, not a mock", async () => {
    demoMock.hasRealSession.mockReturnValue(true);
    clientMock.apiPost.mockResolvedValueOnce({ not: "a proposal" });
    await expect(classifyDuty(REQ as never)).rejects.toBeInstanceOf(ApiError);
  });
});

const SAMPLE_DUTIES = [
  { id: "s1", label: "Persona sample", schedule: "daily 09:00", enabled: true },
];

describe("listDuties — live role_duty list-GET (un-mock schedules panel)", () => {
  it("happy path: maps persisted role_duty rows from the { duties: [...] } envelope", async () => {
    demoMock.hasRealSession.mockReturnValue(true);
    clientMock.apiGet.mockResolvedValueOnce({
      worker_id: "w1",
      duties: [
        { id: "d1", label: "Daily summary", cron: "0 0 6 * * *", enabled: true },
        { id: "d2", label: "Weekly report", cron: "0 0 9 * * 1", enabled: false },
      ],
    });
    const r = await listDuties("w1", () => SAMPLE_DUTIES);
    expect(r.source).toBe("gateway");
    expect(r.duties).toEqual([
      { id: "d1", label: "Daily summary", schedule: "cron: 0 0 6 * * *", enabled: true },
      { id: "d2", label: "Weekly report", schedule: "cron: 0 0 9 * * 1", enabled: false },
    ]);
    expect(clientMock.apiGet).toHaveBeenCalledWith("/api/v1/workers/w1/duties");
  });

  it("happy path: tolerates a bare array body too", async () => {
    clientMock.apiGet.mockResolvedValueOnce([
      { id: "d1", label: "Daily summary", cron: "0 0 6 * * *", enabled: true },
    ]);
    const r = await listDuties("w1");
    expect(r.source).toBe("gateway");
    expect(r.duties).toHaveLength(1);
  });

  it("anonymous: a 404 falls back to the supplied persona samples", async () => {
    clientMock.apiGet.mockRejectedValueOnce(new ApiError(404, "nf", "not_found"));
    const r = await listDuties("w1", () => SAMPLE_DUTIES);
    expect(r.source).toBe("mock");
    expect(r.duties).toEqual(SAMPLE_DUTIES);
  });

  it("REAL user: a 404 surfaces an honest error, NOT a fabricated list", async () => {
    demoMock.hasRealSession.mockReturnValue(true);
    clientMock.apiGet.mockRejectedValueOnce(new ApiError(404, "nf", "not_found"));
    await expect(listDuties("w1", () => SAMPLE_DUTIES)).rejects.toBeInstanceOf(ApiError);
  });

  it("REAL user: a malformed gateway body surfaces an error, not samples", async () => {
    demoMock.hasRealSession.mockReturnValue(true);
    clientMock.apiGet.mockResolvedValueOnce({ not: "a list" });
    await expect(listDuties("w1", () => SAMPLE_DUTIES)).rejects.toBeInstanceOf(ApiError);
  });
});
