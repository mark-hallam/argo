import { describe, expect, it } from "vitest";
import { formatUsdMicros } from "./cost";

describe("formatUsdMicros", () => {
  it("renders zero / non-positive as $0.00 (never blank)", () => {
    expect(formatUsdMicros(0)).toBe("$0.00");
    expect(formatUsdMicros(-5)).toBe("$0.00");
    expect(formatUsdMicros(Number.NaN)).toBe("$0.00");
  });

  it("uses 4 decimals below a dollar so sub-cent costs stay legible", () => {
    expect(formatUsdMicros(1_500)).toBe("$0.0015");
    expect(formatUsdMicros(250_000)).toBe("$0.2500");
  });

  it("uses 2 decimals at or above a dollar", () => {
    expect(formatUsdMicros(1_000_000)).toBe("$1.00");
    expect(formatUsdMicros(4_500_000)).toBe("$4.50");
  });
});
