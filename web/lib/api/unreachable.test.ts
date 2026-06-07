import { describe, expect, it } from "vitest";

import { isMissingOrUnreachable } from "./unreachable";
import { ApiError, GatewayTimeoutError } from "./types";

describe("isMissingOrUnreachable (shared classifier)", () => {
  it("treats a 404 ApiError as missing", () => {
    expect(isMissingOrUnreachable(new ApiError(404, "not found", "not_found"))).toBe(true);
  });

  it("treats a TypeError (CORS / DNS / refused) as unreachable", () => {
    expect(isMissingOrUnreachable(new TypeError("Failed to fetch"))).toBe(true);
  });

  it("does NOT treat other ApiError statuses as missing", () => {
    expect(isMissingOrUnreachable(new ApiError(500, "boom"))).toBe(false);
    expect(isMissingOrUnreachable(new ApiError(403, "forbidden"))).toBe(false);
    expect(isMissingOrUnreachable(new ApiError(400, "bad request"))).toBe(false);
  });

  it("does NOT treat a GatewayTimeoutError as missing — it must PROPAGATE to an honest error", () => {
    // The load-bearing assertion: a timeout is "couldn't load in time", not
    // "no data". Catching it here would swallow it into a false-empty.
    expect(isMissingOrUnreachable(new GatewayTimeoutError(15000, "/api/v1/x"))).toBe(false);
  });

  it("returns false for unrelated errors", () => {
    expect(isMissingOrUnreachable(new Error("something else"))).toBe(false);
    expect(isMissingOrUnreachable(null)).toBe(false);
    expect(isMissingOrUnreachable("string error")).toBe(false);
  });
});
