import { z } from "zod";
import {
  APPROVAL_POLICIES,
  type ApprovalPolicy,
} from "@/lib/api/worker-policies";
import {
  AUTONOMY_LEVELS,
  CHANNEL_KINDS,
  type ChannelKind,
} from "@/lib/api/worker-settings";

/**
 * Validation schemas for the five Worker Settings sections.
 *
 * Each section keeps a thin local form (RHF) and `safeParse`s the
 * snapshot through one of these schemas before the mutation fires.
 * Output types are intentionally the same as the API config types
 * so they slot straight into the existing optimistic-update hooks.
 */

const TIME_HHMM = /^([01]\d|2[0-3]):[0-5]\d$/;

export const channelsFormSchema = z
  .object({
    enabled: z
      .array(z.enum(CHANNEL_KINDS as readonly [ChannelKind, ...ChannelKind[]]))
      .min(1, "Pick at least one channel"),
    primary: z.enum(
      CHANNEL_KINDS as readonly [ChannelKind, ...ChannelKind[]],
    ),
  })
  .superRefine((value, ctx) => {
    if (!value.enabled.includes(value.primary)) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["primary"],
        message: "Primary channel must also be enabled",
      });
    }
  });

export type ChannelsFormValues = z.infer<typeof channelsFormSchema>;

export const quietHoursFormSchema = z
  .object({
    start: z
      .string()
      .regex(TIME_HHMM, "Use 24-hour HH:MM"),
    end: z
      .string()
      .regex(TIME_HHMM, "Use 24-hour HH:MM"),
    timezone: z.string().min(1, "Timezone is required"),
  })
  .superRefine((value, ctx) => {
    if (value.start === value.end) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["end"],
        message: "End time must differ from start",
      });
    }
  });

export type QuietHoursFormValues = z.infer<typeof quietHoursFormSchema>;

/**
 * USD amount field — typed as string at the input boundary (RHF
 * binds <input type="number"> as a string), parsed + bounded into a
 * number at the output boundary so the mutation receives a real
 * value.
 */
const usdAmount = z
  .string()
  .min(1, "Enter a dollar amount")
  .refine((s) => Number.isFinite(Number(s)), "Enter a finite number")
  .refine((s) => Number(s) > 0, "Must be greater than 0")
  .refine((s) => Number(s) <= 1_000_000, "Cap is too large")
  .transform((s) => Number(s));

export const budgetFormSchema = z
  .object({
    dailyCapUsd: usdAmount,
    monthlyCapUsd: usdAmount,
  })
  .superRefine((value, ctx) => {
    if (value.monthlyCapUsd < value.dailyCapUsd) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["monthlyCapUsd"],
        message: "Monthly cap must be ≥ daily cap",
      });
    }
  });

export type BudgetFormValues = z.infer<typeof budgetFormSchema>;

export const autonomyFormSchema = z.object({
  level: z.enum(AUTONOMY_LEVELS as readonly [
    (typeof AUTONOMY_LEVELS)[number],
    ...(typeof AUTONOMY_LEVELS)[number][],
  ]),
});

export type AutonomyFormValues = z.infer<typeof autonomyFormSchema>;

export const approvalPolicyFormSchema = z.object({
  policy: z.enum(
    APPROVAL_POLICIES as readonly [ApprovalPolicy, ...ApprovalPolicy[]],
  ),
  operatorNote: z
    .string()
    .trim()
    .min(1, "Add a short note — it shows up in the audit trail")
    .max(1024, "Note is too long (1024 char max)"),
});

export type ApprovalPolicyFormValues = z.infer<typeof approvalPolicyFormSchema>;
