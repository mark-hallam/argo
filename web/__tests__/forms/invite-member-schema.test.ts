import { describe, expect, it } from "vitest";
import { inviteMemberSchema } from "@/app/(app)/settings/members/schemas";

describe("inviteMemberSchema", () => {
  it("trims + lower-cases a valid email", () => {
    const result = inviteMemberSchema.safeParse({
      email: " Teammate@COMPANY.com ",
      role: "user",
      cohortScope: null,
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.email).toBe("teammate@company.com");
    }
  });

  it("rejects malformed emails", () => {
    expect(
      inviteMemberSchema.safeParse({
        email: "not-an-email",
        role: "user",
        cohortScope: null,
      }).success,
    ).toBe(false);
  });

  it("rejects emails over 254 chars", () => {
    expect(
      inviteMemberSchema.safeParse({
        email: `${"x".repeat(250)}@co.io`,
        role: "user",
        cohortScope: null,
      }).success,
    ).toBe(false);
  });

  it("rejects unknown roles", () => {
    expect(
      inviteMemberSchema.safeParse({
        email: "fine@example.com",
        role: "superuser",
        cohortScope: null,
      }).success,
    ).toBe(false);
  });

  it("accepts a cohort scope array or null", () => {
    expect(
      inviteMemberSchema.safeParse({
        email: "fine@example.com",
        role: "user",
        cohortScope: ["sales", "support"],
      }).success,
    ).toBe(true);
  });
});
