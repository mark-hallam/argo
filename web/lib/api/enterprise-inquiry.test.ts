import { afterEach, describe, expect, it, vi } from "vitest";
import { ApiError } from "./types";

// Mock the gateway client so the test exercises enterprise-inquiry.ts logic,
// not fetch.
vi.mock("./client", () => ({
  apiPost: vi.fn(),
}));

import { apiPost } from "./client";
import { isValidEmail, submitEnterpriseInquiry } from "./enterprise-inquiry";

const apiPostMock = vi.mocked(apiPost);

afterEach(() => {
  vi.clearAllMocks();
});

describe("lib/api/enterprise-inquiry — isValidEmail", () => {
  it("accepts a normal address and rejects obvious junk", () => {
    expect(isValidEmail("you@company.com")).toBe(true);
    expect(isValidEmail("  spaced@ok.io  ")).toBe(true);
    expect(isValidEmail("nope")).toBe(false);
    expect(isValidEmail("missing@tld")).toBe(false);
    expect(isValidEmail("")).toBe(false);
  });
});

describe("lib/api/enterprise-inquiry — submitEnterpriseInquiry", () => {
  const ok = { company: "Acme", name: "Alice", email: "alice@acme.com" };

  it("rejects a missing company WITHOUT calling the backend", async () => {
    await expect(
      submitEnterpriseInquiry({ ...ok, company: "  " }),
    ).rejects.toBeInstanceOf(ApiError);
    expect(apiPostMock).not.toHaveBeenCalled();
  });

  it("rejects a missing name WITHOUT calling the backend", async () => {
    await expect(
      submitEnterpriseInquiry({ ...ok, name: "" }),
    ).rejects.toBeInstanceOf(ApiError);
    expect(apiPostMock).not.toHaveBeenCalled();
  });

  it("rejects an invalid email WITHOUT calling the backend", async () => {
    await expect(
      submitEnterpriseInquiry({ ...ok, email: "nope" }),
    ).rejects.toBeInstanceOf(ApiError);
    expect(apiPostMock).not.toHaveBeenCalled();
  });

  it("posts trimmed required fields to /api/v1/enterprise-inquiry", async () => {
    apiPostMock.mockResolvedValueOnce({ queued: true });
    const res = await submitEnterpriseInquiry({
      company: "  Acme  ",
      name: " Alice ",
      email: "  alice@acme.com ",
    });
    expect(res.queued).toBe(true);
    expect(apiPostMock).toHaveBeenCalledWith("/api/v1/enterprise-inquiry", {
      company: "Acme",
      name: "Alice",
      email: "alice@acme.com",
    });
  });

  it("includes optional seats + message only when non-empty", async () => {
    apiPostMock.mockResolvedValueOnce({ queued: true });
    await submitEnterpriseInquiry({
      ...ok,
      seats: " 250 ",
      message: "  Q3 rollout ",
    });
    expect(apiPostMock).toHaveBeenCalledWith("/api/v1/enterprise-inquiry", {
      company: "Acme",
      name: "Alice",
      email: "alice@acme.com",
      seats: "250",
      message: "Q3 rollout",
    });
  });

  it("propagates a backend error as an ApiError — no fake success", async () => {
    apiPostMock.mockRejectedValueOnce(new ApiError(500, "boom"));
    await expect(submitEnterpriseInquiry(ok)).rejects.toMatchObject({
      status: 500,
    });
  });

  it("treats a 2xx with an empty/odd body as queued", async () => {
    apiPostMock.mockResolvedValueOnce(undefined);
    const res = await submitEnterpriseInquiry(ok);
    expect(res.queued).toBe(true);
  });
});
