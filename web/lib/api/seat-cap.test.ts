import { describe, expect, it } from "vitest";
import { ApiError } from "./types";
import { PRO } from "@/lib/pricing/tiers";
import {
  SEAT_CAP_ERROR_CODE,
  seatCapErrorInfo,
  seatCapMessage,
} from "./seat-cap";

function seatCapError(
  body: Record<string, unknown> = {
    error: SEAT_CAP_ERROR_CODE,
    tier: "free",
    used: 1,
    max_workers: 1,
    tier_upsell_url: "https://skycot.com/upgrade",
  },
): ApiError {
  return new ApiError(403, "Forbidden", SEAT_CAP_ERROR_CODE, body);
}

describe("seatCapErrorInfo", () => {
  it("extracts the structured fields from a seat-cap 403", () => {
    const info = seatCapErrorInfo(seatCapError());
    expect(info).toEqual({
      tier: "free",
      used: 1,
      maxWorkers: 1,
      upsellUrl: "https://skycot.com/upgrade",
    });
  });

  it("returns null for a generic RBAC forbidden 403", () => {
    const rbac = new ApiError(403, "no capability", "forbidden", {
      error: { code: "forbidden", message: "no capability" },
    });
    expect(seatCapErrorInfo(rbac)).toBeNull();
  });

  it("returns null for non-403 statuses even with the seat-cap code", () => {
    expect(seatCapErrorInfo(new ApiError(500, "x", SEAT_CAP_ERROR_CODE))).toBeNull();
  });

  it("returns null for non-ApiError values", () => {
    expect(seatCapErrorInfo(new Error("boom"))).toBeNull();
    expect(seatCapErrorInfo(null)).toBeNull();
    expect(seatCapErrorInfo("Forbidden")).toBeNull();
  });

  it("falls back to safe defaults when the body is missing fields", () => {
    const info = seatCapErrorInfo(new ApiError(403, "Forbidden", SEAT_CAP_ERROR_CODE));
    expect(info).toEqual({ tier: "free", used: undefined, maxWorkers: 1, upsellUrl: undefined });
  });
});

describe("seatCapMessage", () => {
  it("nudges a Free-tier tenant to upgrade to Pro with the canonical Pro cap", () => {
    const msg = seatCapMessage({ tier: "free", maxWorkers: 1 });
    expect(msg).toContain("Free plan's 1-teammate limit");
    expect(msg).toContain(`upgrade to Pro to hire up to ${PRO.max_workers} teammates`);
  });

  it("points a Pro/Enterprise tenant at adding seats, not upgrading to Pro", () => {
    const msg = seatCapMessage({ tier: "pro", maxWorkers: PRO.max_workers });
    expect(msg).toContain("Pro plan");
    expect(msg).not.toContain("upgrade to Pro");
    expect(msg).toContain("add more seats");
  });
});
