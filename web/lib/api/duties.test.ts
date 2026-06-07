import { afterEach, describe, expect, it, vi } from "vitest";

// Control the gateway client + the real-session flag so we can pin the
// mock-leak class gate: a REAL signed-in user must NEVER be served a
// fabricated schedule on a 404 / malformed body — they get an honest error.
// Anonymous dev/demo keeps the deterministic mock so the wizard is testable
// with the gateway offline. The gateway happy path is unchanged for both.
const clientMock = vi.hoisted(() => ({ apiPost: vi.fn(), apiPatch: vi.fn() }));
vi.mock("@/lib/api/client", () => clientMock);

const demoMock = vi.hoisted(() => ({ hasRealSession: vi.fn(() => false) }));
vi.mock("@/lib/skycot/demo-mode", () => demoMock);

import { classifyDuty } from "./duties";
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
