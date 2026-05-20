import { describe, expect, it } from "vitest";
import { addFactFormSchema } from "@/components/skycot/worker/schemas";

describe("addFactFormSchema", () => {
  it("trims and accepts a non-empty fact", () => {
    const result = addFactFormSchema.safeParse({
      text: "  Refunds under $50 auto-approve.  ",
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.text).toBe("Refunds under $50 auto-approve.");
    }
  });

  it("rejects whitespace-only text (double-submit guard)", () => {
    expect(addFactFormSchema.safeParse({ text: "   " }).success).toBe(false);
  });

  it("rejects empty input", () => {
    expect(addFactFormSchema.safeParse({ text: "" }).success).toBe(false);
  });

  it("rejects pathological payloads (> 2000 chars)", () => {
    expect(
      addFactFormSchema.safeParse({ text: "a".repeat(2001) }).success,
    ).toBe(false);
  });
});
