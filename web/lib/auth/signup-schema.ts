import { z } from "zod";

/**
 * Email validation for the magic-link signup form. Trim + lowercase
 * before validation so trailing whitespace and casing don't produce
 * spurious rejections — the gateway treats addresses case-insensitively.
 */
export const signupSchema = z.object({
  email: z
    .string()
    .trim()
    .toLowerCase()
    .min(1, "Email is required")
    .max(254, "Email is too long")
    .email("Enter a valid email address"),
});

export type SignupInput = z.infer<typeof signupSchema>;

export const verifySchema = z.object({
  token: z.string().min(8, "Token is missing or malformed"),
});

export type VerifyInput = z.infer<typeof verifySchema>;
