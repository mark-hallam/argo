import { afterEach, describe, expect, it, vi } from "vitest";

// Mock-leak class gate: a REAL signed-in user must never be served a
// fabricated retrain proposal on a 404 / malformed body — they get an honest
// error. Anonymous dev/demo keeps the deterministic mock.
const clientMock = vi.hoisted(() => ({
  apiPost: vi.fn(),
  apiGet: vi.fn(),
  apiDelete: vi.fn(),
}));
vi.mock("@/lib/api/client", () => clientMock);

const demoMock = vi.hoisted(() => ({ hasRealSession: vi.fn(() => false) }));
vi.mock("@/lib/skycot/demo-mode", () => demoMock);

import { classifyRetrain } from "./retrain";
import { ApiError } from "./types";

const REQ = {
  workerId: "w1",
  input: "stop emailing customers on weekends",
  triggerContext: undefined,
};

afterEach(() => {
  vi.clearAllMocks();
  demoMock.hasRealSession.mockReturnValue(false);
});

describe("retrain — real-session NO-THEATER gate (mock-leak class)", () => {
  it("anonymous: a 404 falls back to a mock proposal (dev convenience)", async () => {
    clientMock.apiPost.mockRejectedValueOnce(new ApiError(404, "nf", "not_found"));
    const r = await classifyRetrain(REQ as never);
    expect(r.source).toBe("mock");
  });

  it("REAL user: a 404 surfaces an honest error, NOT a fabricated proposal", async () => {
    demoMock.hasRealSession.mockReturnValue(true);
    clientMock.apiPost.mockRejectedValueOnce(new ApiError(404, "nf", "not_found"));
    await expect(classifyRetrain(REQ as never)).rejects.toBeInstanceOf(ApiError);
  });

  it("REAL user: a malformed gateway body surfaces an error, not a mock", async () => {
    demoMock.hasRealSession.mockReturnValue(true);
    clientMock.apiPost.mockResolvedValueOnce({ not: "a proposal" });
    await expect(classifyRetrain(REQ as never)).rejects.toBeInstanceOf(ApiError);
  });
});
