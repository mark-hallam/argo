import { z } from "zod";

/**
 * Schema for the "Add a fact" dialog. The text is trimmed before
 * validation so trailing whitespace doesn't masquerade as content,
 * and bounded to avoid pathological gateway payloads.
 */
export const addFactFormSchema = z.object({
  text: z
    .string()
    .trim()
    .min(1, "Write the fact you want to teach")
    .max(2000, "Fact is too long (2000 char max)"),
});

export type AddFactFormValues = z.infer<typeof addFactFormSchema>;
