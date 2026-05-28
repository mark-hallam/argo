import { describe, expect, it } from "vitest";
import { formatVersion } from "./version";

describe("formatVersion", () => {
  it("formats an integer count as v<n>", () => {
    expect(formatVersion(3)).toBe("v3");
    expect(formatVersion(1)).toBe("v1");
    expect(formatVersion(42)).toBe("v42");
  });

  it("collapses a semver string to its major version", () => {
    expect(formatVersion("1.0.0")).toBe("v1");
    expect(formatVersion("2.4.1")).toBe("v2");
  });

  it("coerces a numeric string to v<n>", () => {
    expect(formatVersion("3")).toBe("v3");
    expect(formatVersion("12")).toBe("v12");
  });

  it("renders 0 as v0 (unauthored workflow edge case)", () => {
    expect(formatVersion(0)).toBe("v0");
    expect(formatVersion("0")).toBe("v0");
  });

  it("strips an existing v / V prefix without double-prefixing", () => {
    expect(formatVersion("v3")).toBe("v3");
    expect(formatVersion("V6")).toBe("v6");
  });

  it("clamps negative or non-finite numbers to v0", () => {
    expect(formatVersion(-1)).toBe("v0");
    expect(formatVersion(Number.NaN)).toBe("v0");
    expect(formatVersion(Infinity)).toBe("v0");
  });

  it("falls back to v0 for unparseable strings", () => {
    expect(formatVersion("")).toBe("v0");
    expect(formatVersion("draft")).toBe("v0");
  });

  it("truncates a fractional number to its integer part", () => {
    expect(formatVersion(3.7)).toBe("v3");
  });
});
