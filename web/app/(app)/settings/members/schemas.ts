import { z } from "zod";

/**
 * Validation schema for the Invite-teammate modal. Mirrors the
 * server-side rules: trimmed lower-cased work email, role drawn
 * from the canonical 4-tier ladder, optional cohort scope.
 */

const MEMBER_ROLES = ["owner", "admin", "user", "guest"] as const;

export const inviteMemberSchema = z.object({
  email: z
    .string()
    .trim()
    .toLowerCase()
    .min(1, "Work email is required")
    .max(254, "Email is too long")
    .email("Enter a valid work email"),
  role: z.enum(MEMBER_ROLES),
  cohortScope: z.array(z.string()).nullable(),
});

export type InviteMemberFormValues = z.infer<typeof inviteMemberSchema>;
