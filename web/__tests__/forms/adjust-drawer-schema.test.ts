import { describe, expect, it } from "vitest";
import { adjustDrawerFormSchema } from "@/components/skycot/hire/schemas";

describe("adjustDrawerFormSchema", () => {
  it("trims and accepts a non-empty adjustment", () => {
    const result = adjustDrawerFormSchema.safeParse({
      text: "  Make her more formal  ",
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.text).toBe("Make her more formal");
    }
  });

  it("rejects whitespace-only adjustments (double-submit guard)", () => {
    expect(adjustDrawerFormSchema.safeParse({ text: "   " }).success).toBe(
      false,
    );
  });

  it("rejects empty input", () => {
    expect(adjustDrawerFormSchema.safeParse({ text: "" }).success).toBe(false);
  });

  it("caps adjustment length", () => {
    expect(
      adjustDrawerFormSchema.safeParse({ text: "x".repeat(2001) }).success,
    ).toBe(false);
  });
});
