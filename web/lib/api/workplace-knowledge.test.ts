/**
 * Strict-production behavior for the workplace-knowledge API client.
 *
 * Design doc §7 Pillar 1 acceptance #1: when `NODE_ENV === "production"`
 * AND `NEXT_PUBLIC_ENV === "production"`, the client must propagate
 * gateway errors (notably 404) instead of falling back to `SAMPLE_KNOWLEDGE`.
 * Outside that envelope the dev fallback returns the fixture so the
 * page renders meaningfully without a running gateway.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  isStrictProduction,
  listKnowledgeDocs,
  SAMPLE_KNOWLEDGE,
} from "./workplace-knowledge";
import { ApiError } from "./types";

const apiGetMock = vi.hoisted(() => vi.fn());

vi.mock("./client", async () => {
  const mod = await vi.importActual<typeof import("./client")>("./client");
  return {
    ...mod,
    apiGet: apiGetMock,
  };
});

beforeEach(() => {
  apiGetMock.mockReset();
  vi.unstubAllEnvs();
});

afterEach(() => {
  vi.unstubAllEnvs();
});

describe("isStrictProduction", () => {
  it("returns true only when both NODE_ENV and NEXT_PUBLIC_ENV are production", () => {
    vi.stubEnv("NODE_ENV", "production");
    vi.stubEnv("NEXT_PUBLIC_ENV", "production");
    expect(isStrictProduction()).toBe(true);
  });

  it("returns false in any other env combination", () => {
    vi.stubEnv("NODE_ENV", "production");
    vi.stubEnv("NEXT_PUBLIC_ENV", "preview");
    expect(isStrictProduction()).toBe(false);

    vi.stubEnv("NODE_ENV", "development");
    vi.stubEnv("NEXT_PUBLIC_ENV", "production");
    expect(isStrictProduction()).toBe(false);
  });
});

describe("listKnowledgeDocs strict-mode behavior", () => {
  it("falls back to SAMPLE_KNOWLEDGE on 404 in dev", async () => {
    vi.stubEnv("NODE_ENV", "test");
    vi.stubEnv("NEXT_PUBLIC_ENV", "test");
    apiGetMock.mockRejectedValueOnce(new ApiError(404, "not found"));
    const result = await listKnowledgeDocs();
    expect(result.source).toBe("mock-offline");
    expect(result.docs).toEqual(SAMPLE_KNOWLEDGE);
  });

  it("propagates 404 as an ApiError in strict-production mode (no fallback)", async () => {
    vi.stubEnv("NODE_ENV", "production");
    vi.stubEnv("NEXT_PUBLIC_ENV", "production");
    apiGetMock.mockRejectedValueOnce(new ApiError(404, "not found"));
    await expect(listKnowledgeDocs()).rejects.toBeInstanceOf(ApiError);
  });

  it("returns gateway data when the call succeeds", async () => {
    vi.stubEnv("NODE_ENV", "test");
    vi.stubEnv("NEXT_PUBLIC_ENV", "test");
    apiGetMock.mockResolvedValueOnce({ count: 0, items: [] });
    const result = await listKnowledgeDocs();
    expect(result.source).toBe("gateway");
    expect(result.docs).toEqual([]);
  });
});
