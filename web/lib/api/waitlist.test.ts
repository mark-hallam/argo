import { afterEach, describe, expect, it, vi } from "vitest";
import { ApiError } from "./types";

// Mock the gateway client so the test exercises waitlist.ts logic, not fetch.
vi.mock("./client", () => ({
  apiPost: vi.fn(),
}));

import { apiPost } from "./client";
import { isValidEmail, joinWaitlist } from "./waitlist";

const apiPostMock = vi.mocked(apiPost);

afterEach(() => {
  vi.clearAllMocks();
});

describe("lib/api/waitlist — isValidEmail", () => {
  it("accepts a normal address and rejects obvious junk", () => {
    expect(isValidEmail("you@company.com")).toBe(true);
    expect(isValidEmail("  spaced@ok.io  ")).toBe(true);
    expect(isValidEmail("nope")).toBe(false);
    expect(isValidEmail("missing@tld")).toBe(false);
    expect(isValidEmail("")).toBe(false);
  });
});

describe("lib/api/waitlist — joinWaitlist", () => {
  it("rejects an invalid email WITHOUT calling the backend", async () => {
    await expect(joinWaitlist({ email: "nope" })).rejects.toBeInstanceOf(ApiError);
    expect(apiPostMock).not.toHaveBeenCalled();
  });

  it("posts a trimmed email + source to /api/v1/waitlist", async () => {
    apiPostMock.mockResolvedValueOnce({ queued: true });
    const res = await joinWaitlist({ email: "  founder@acme.io ", source: "landing-hero" });
    expect(res.queued).toBe(true);
    expect(apiPostMock).toHaveBeenCalledWith("/api/v1/waitlist", {
      email: "founder@acme.io",
      source: "landing-hero",
    });
  });

  it("propagates the backend 404 (route not built yet) as an ApiError — no fake success", async () => {
    apiPostMock.mockRejectedValueOnce(new ApiError(404, "Not Found"));
    await expect(joinWaitlist({ email: "founder@acme.io" })).rejects.toMatchObject({
      status: 404,
    });
  });

  it("treats a 2xx with an empty/odd body as queued", async () => {
    apiPostMock.mockResolvedValueOnce(undefined);
    const res = await joinWaitlist({ email: "founder@acme.io" });
    expect(res.queued).toBe(true);
  });
});
