import { describe, expect, it } from "vitest";
import { formatKpi } from "./kpi";

describe("formatKpi", () => {
  it("formats a finite number through the provided formatter", () => {
    expect(formatKpi(42, (n) => String(n))).toBe("42");
    expect(formatKpi(0.5, (n) => `${Math.round(n * 100)}%`)).toBe("50%");
  });

  it("renders 0 as the formatted zero rather than the fallback", () => {
    expect(formatKpi(0, String)).toBe("0");
    expect(formatKpi(0, (n) => `$${n}`)).toBe("$0");
  });

  it("returns the fallback for null / undefined", () => {
    expect(formatKpi(null, String)).toBe("—");
    expect(formatKpi(undefined, String)).toBe("—");
  });

  it("accepts a custom fallback", () => {
    expect(formatKpi(null, String, "n/a")).toBe("n/a");
  });

  it("guards against NaN and Infinity", () => {
    expect(formatKpi(Number.NaN, String)).toBe("—");
    expect(formatKpi(Infinity, String)).toBe("—");
  });
});
