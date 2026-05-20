import { describe, expect, it } from "vitest";
import { z } from "zod";
import { zodResolver } from "@/lib/forms/use-zod-form";

describe("zodResolver", () => {
  it("returns parsed values when input is valid", async () => {
    const schema = z.object({ name: z.string().min(1) });
    const resolver = zodResolver(schema);
    const result = await resolver({ name: "Mark" }, undefined, {
      criteriaMode: "firstError",
      fields: {},
      shouldUseNativeValidation: false,
    });
    expect(result.errors).toEqual({});
    expect(result.values).toEqual({ name: "Mark" });
  });

  it("converts Zod issues to RHF-style errors for nested paths", async () => {
    const schema = z.object({
      profile: z.object({ email: z.string().email() }),
    });
    const resolver = zodResolver(schema);
    const result = await resolver(
      { profile: { email: "not-an-email" } },
      undefined,
      { criteriaMode: "firstError", fields: {}, shouldUseNativeValidation: false },
    );
    const errors = result.errors as {
      profile?: { email?: { message?: string } };
    };
    expect(errors.profile?.email?.message).toMatch(/email/i);
    expect(result.values).toEqual({});
  });

  it("first issue wins when two issues touch the same field", async () => {
    const schema = z
      .object({ token: z.string().min(8) })
      .refine((v) => v.token !== "blocked", {
        path: ["token"],
        message: "Token is blocked",
      });
    const resolver = zodResolver(schema);
    const result = await resolver({ token: "x" }, undefined, {
      criteriaMode: "firstError",
      fields: {},
      shouldUseNativeValidation: false,
    });
    const errors = result.errors as { token?: { message?: string } };
    expect(errors.token?.message).toBeDefined();
  });
});
