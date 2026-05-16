import { describe, expect, it } from "vitest";
import { signupSchema, verifySchema } from "@/lib/auth/signup-schema";

describe("signupSchema", () => {
  it("accepts a valid email and lowercases it", () => {
    const result = signupSchema.safeParse({ email: " Mark@Example.com " });
    expect(result.success).toBe(true);
    if (result.success) expect(result.data.email).toBe("mark@example.com");
  });

  it("rejects malformed addresses", () => {
    expect(signupSchema.safeParse({ email: "not-an-email" }).success).toBe(false);
    expect(signupSchema.safeParse({ email: "" }).success).toBe(false);
  });

  it("rejects overly long addresses", () => {
    const long = `${"a".repeat(250)}@b.co`;
    expect(signupSchema.safeParse({ email: long }).success).toBe(false);
  });
});

describe("verifySchema", () => {
  it("requires a token of plausible length", () => {
    expect(verifySchema.safeParse({ token: "abcd" }).success).toBe(false);
    expect(verifySchema.safeParse({ token: "abcd1234efgh" }).success).toBe(true);
  });
});
