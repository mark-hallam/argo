import { describe, expect, it } from "vitest";
import { usagePercent } from "./usage";

describe("usagePercent", () => {
  it("returns the clamped fraction of the cap consumed", () => {
    expect(usagePercent(0, 100)).toBe(0);
    expect(usagePercent(50, 100)).toBe(50);
    expect(usagePercent(4000, 5000)).toBe(80);
    expect(usagePercent(100, 100)).toBe(100);
  });

  it("clamps over-cap usage to 100", () => {
    expect(usagePercent(250, 100)).toBe(100);
  });

  it("returns 0 for the unlimited tier (limit === 0) — no percentage of unlimited", () => {
    expect(usagePercent(1200, 0)).toBe(0);
  });

  it("returns 0 for non-finite or negative inputs (never a fabricated fill)", () => {
    expect(usagePercent(Number.NaN, 100)).toBe(0);
    expect(usagePercent(50, Number.NaN)).toBe(0);
    expect(usagePercent(50, -10)).toBe(0);
  });
});
