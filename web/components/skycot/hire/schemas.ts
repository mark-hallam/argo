import { z } from "zod";

/**
 * Schema for the Step-4 Adjust drawer. Trim guards against
 * whitespace-only "instructions" and bounds the payload at a
 * sensible size (LLM context budget for the redraft).
 */
export const adjustDrawerFormSchema = z.object({
  text: z
    .string()
    .trim()
    .min(1, "Tell us what to change")
    .max(2000, "Adjustment is too long (2000 char max)"),
});

export type AdjustDrawerFormValues = z.infer<typeof adjustDrawerFormSchema>;
