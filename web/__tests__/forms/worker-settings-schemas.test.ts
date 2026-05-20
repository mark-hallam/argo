import { describe, expect, it } from "vitest";
import {
  approvalPolicyFormSchema,
  autonomyFormSchema,
  budgetFormSchema,
  channelsFormSchema,
  quietHoursFormSchema,
} from "@/components/skycot/worker/settings/schemas";

describe("channelsFormSchema", () => {
  it("accepts a non-empty enabled list with the primary enabled", () => {
    const result = channelsFormSchema.safeParse({
      enabled: ["slack", "email"],
      primary: "slack",
    });
    expect(result.success).toBe(true);
  });

  it("rejects an empty enabled list", () => {
    expect(
      channelsFormSchema.safeParse({ enabled: [], primary: "slack" }).success,
    ).toBe(false);
  });

  it("rejects a primary that is not in enabled", () => {
    const result = channelsFormSchema.safeParse({
      enabled: ["slack"],
      primary: "email",
    });
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.issues.some((i) => i.path[0] === "primary")).toBe(true);
    }
  });
});

describe("quietHoursFormSchema", () => {
  it("accepts a valid HH:MM range with timezone", () => {
    const result = quietHoursFormSchema.safeParse({
      start: "21:00",
      end: "08:00",
      timezone: "America/Los_Angeles",
    });
    expect(result.success).toBe(true);
  });

  it("rejects malformed times", () => {
    expect(
      quietHoursFormSchema.safeParse({
        start: "9pm",
        end: "08:00",
        timezone: "UTC",
      }).success,
    ).toBe(false);
  });

  it("rejects a missing timezone", () => {
    expect(
      quietHoursFormSchema.safeParse({
        start: "21:00",
        end: "08:00",
        timezone: "",
      }).success,
    ).toBe(false);
  });

  it("rejects when start equals end (zero-length window)", () => {
    expect(
      quietHoursFormSchema.safeParse({
        start: "09:00",
        end: "09:00",
        timezone: "UTC",
      }).success,
    ).toBe(false);
  });
});

describe("budgetFormSchema", () => {
  it("coerces numeric strings and accepts positive caps", () => {
    const result = budgetFormSchema.safeParse({
      dailyCapUsd: "10",
      monthlyCapUsd: "150",
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.dailyCapUsd).toBe(10);
      expect(result.data.monthlyCapUsd).toBe(150);
    }
  });

  it("rejects zero or negative caps", () => {
    expect(
      budgetFormSchema.safeParse({ dailyCapUsd: "0", monthlyCapUsd: "150" })
        .success,
    ).toBe(false);
    expect(
      budgetFormSchema.safeParse({ dailyCapUsd: "10", monthlyCapUsd: "-5" })
        .success,
    ).toBe(false);
  });

  it("rejects monthly cap below daily cap", () => {
    expect(
      budgetFormSchema.safeParse({ dailyCapUsd: "50", monthlyCapUsd: "30" })
        .success,
    ).toBe(false);
  });

  it("rejects non-numeric input", () => {
    expect(
      budgetFormSchema.safeParse({ dailyCapUsd: "abc", monthlyCapUsd: "150" })
        .success,
    ).toBe(false);
  });
});

describe("autonomyFormSchema", () => {
  it("accepts each canonical level", () => {
    for (const level of ["conservative", "balanced", "aggressive"] as const) {
      expect(autonomyFormSchema.safeParse({ level }).success).toBe(true);
    }
  });

  it("rejects unknown levels", () => {
    expect(autonomyFormSchema.safeParse({ level: "wild" }).success).toBe(false);
  });
});

describe("approvalPolicyFormSchema", () => {
  it("accepts a canonical policy with a trimmed non-empty note", () => {
    const result = approvalPolicyFormSchema.safeParse({
      policy: "always-ask",
      operatorNote: "  audit-evidence test  ",
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.operatorNote).toBe("audit-evidence test");
    }
  });

  it("rejects an empty / whitespace-only note", () => {
    expect(
      approvalPolicyFormSchema.safeParse({
        policy: "always-ask",
        operatorNote: "   ",
      }).success,
    ).toBe(false);
  });

  it("rejects notes that exceed the audit cap", () => {
    expect(
      approvalPolicyFormSchema.safeParse({
        policy: "always-ask",
        operatorNote: "a".repeat(1025),
      }).success,
    ).toBe(false);
  });

  it("rejects unknown policies", () => {
    expect(
      approvalPolicyFormSchema.safeParse({
        policy: "yolo",
        operatorNote: "fine",
      }).success,
    ).toBe(false);
  });
});
