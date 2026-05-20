/**
 * Shared types + zod schemas for workflow trigger configuration forms.
 *
 * The four trigger kinds with a tailored UI (webhook / schedule / email /
 * form) each own a discriminated-union variant of `TriggerConfig`. The
 * builder's step-config save path consumes a `TriggerConfig` and serializes
 * it into the workflow's TOML representation in a follow-up brief.
 *
 * Manual + slack triggers are recognised here so the dispatcher in
 * `TriggerConfigDrawer` can route them to a "no extra config" message
 * rather than crashing.
 */
import { z } from "zod";

export type TriggerKind = "manual" | "schedule" | "webhook" | "email" | "form" | "slack";

export const TRIGGER_KINDS: TriggerKind[] = ["manual", "schedule", "webhook", "email", "form", "slack"];

// --- webhook ---------------------------------------------------------------

export const WEBHOOK_METHODS = ["POST", "GET", "PUT", "PATCH", "DELETE"] as const;
export type WebhookMethod = (typeof WEBHOOK_METHODS)[number];

export const webhookConfigSchema = z.object({
  kind: z.literal("webhook"),
  workflowId: z.string().min(1),
  secretSuffix: z.string().min(8),
  allowedMethods: z.array(z.enum(WEBHOOK_METHODS)).min(1, "Pick at least one method."),
  authMode: z.enum(["none", "header"]),
  authHeaderName: z.string().optional(),
  authHeaderValue: z.string().optional(),
}).superRefine((cfg, ctx) => {
  if (cfg.authMode === "header") {
    if (!cfg.authHeaderName || !cfg.authHeaderName.trim()) {
      ctx.addIssue({ code: "custom", path: ["authHeaderName"], message: "Header name is required." });
    }
    if (!cfg.authHeaderValue || !cfg.authHeaderValue.trim()) {
      ctx.addIssue({ code: "custom", path: ["authHeaderValue"], message: "Header value is required." });
    }
  }
});

export type WebhookConfig = z.infer<typeof webhookConfigSchema>;

export function webhookUrl(cfg: { workflowId: string; secretSuffix: string }): string {
  return `https://api.skycot.com/v1/webhooks/${cfg.workflowId}/${cfg.secretSuffix}`;
}

// --- schedule --------------------------------------------------------------

export const SCHEDULE_MODES = ["minute", "hour", "day", "week", "month", "custom"] as const;
export type ScheduleMode = (typeof SCHEDULE_MODES)[number];

export const scheduleConfigSchema = z.object({
  kind: z.literal("schedule"),
  mode: z.enum(SCHEDULE_MODES),
  // mode-specific fields, all optional at the type level — the form
  // enforces required-ness contextually
  hour: z.number().int().min(0).max(23).optional(),
  minute: z.number().int().min(0).max(59).optional(),
  daysOfWeek: z.array(z.number().int().min(0).max(6)).optional(),
  dayOfMonth: z.number().int().min(1).max(31).optional(),
  cronExpression: z.string().optional(),
  timezone: z.string().min(1, "Pick a timezone."),
}).superRefine((cfg, ctx) => {
  if (cfg.mode === "custom" && (!cfg.cronExpression || !cfg.cronExpression.trim())) {
    ctx.addIssue({ code: "custom", path: ["cronExpression"], message: "Cron expression is required." });
  }
  if (cfg.mode === "week" && (!cfg.daysOfWeek || cfg.daysOfWeek.length === 0)) {
    ctx.addIssue({ code: "custom", path: ["daysOfWeek"], message: "Pick at least one day." });
  }
});

export type ScheduleConfig = z.infer<typeof scheduleConfigSchema>;

// --- email -----------------------------------------------------------------

export const emailConfigSchema = z.object({
  kind: z.literal("email"),
  imapHost: z.string().min(1, "Server is required."),
  imapPort: z.number().int().min(1).max(65535),
  imapUsername: z.string().min(1, "Username is required."),
  imapPassword: z.string().min(1, "Password is required."),
  useTls: z.boolean(),
  filterFrom: z.string().optional(),
  filterSubject: z.string().optional(),
  filterHasAttachment: z.boolean(),
});

export type EmailConfig = z.infer<typeof emailConfigSchema>;

// --- form ------------------------------------------------------------------

export const FORM_FIELD_TYPES = ["text", "textarea", "email", "number", "date", "select", "checkbox", "file"] as const;
export type FormFieldType = (typeof FORM_FIELD_TYPES)[number];

export const formFieldSchema = z.object({
  id: z.string().min(1),
  label: z.string().min(1, "Label is required."),
  type: z.enum(FORM_FIELD_TYPES),
  required: z.boolean(),
  placeholder: z.string().optional(),
});

export type FormFieldDef = z.infer<typeof formFieldSchema>;

export const formConfigSchema = z.object({
  kind: z.literal("form"),
  workflowId: z.string().min(1),
  fields: z.array(formFieldSchema).min(1, "Add at least one field."),
  visibility: z.enum(["public", "private"]),
  submitLabel: z.string().min(1, "Submit label is required."),
  successMessage: z.string().min(1, "Success message is required."),
});

export type FormConfig = z.infer<typeof formConfigSchema>;

export function formUrl(cfg: { workflowId: string }): string {
  return `https://skycot.com/forms/${cfg.workflowId}`;
}

// --- union -----------------------------------------------------------------

export type TriggerConfig = WebhookConfig | ScheduleConfig | EmailConfig | FormConfig;

// --- defaults --------------------------------------------------------------

export function defaultWebhookConfig(workflowId: string): WebhookConfig {
  return {
    kind: "webhook",
    workflowId,
    secretSuffix: generateSecretSuffix(),
    allowedMethods: ["POST"],
    authMode: "none",
    authHeaderName: undefined,
    authHeaderValue: undefined,
  };
}

export function defaultScheduleConfig(timezone: string): ScheduleConfig {
  return {
    kind: "schedule",
    mode: "day",
    hour: 9,
    minute: 0,
    daysOfWeek: [1, 2, 3, 4, 5],
    dayOfMonth: 1,
    cronExpression: "0 9 * * 1-5",
    timezone,
  };
}

export function defaultEmailConfig(): EmailConfig {
  return {
    kind: "email",
    imapHost: "",
    imapPort: 993,
    imapUsername: "",
    imapPassword: "",
    useTls: true,
    filterFrom: "",
    filterSubject: "",
    filterHasAttachment: false,
  };
}

export function defaultFormConfig(workflowId: string): FormConfig {
  return {
    kind: "form",
    workflowId,
    fields: [
      { id: "f1", label: "Your name", type: "text", required: true, placeholder: "Jane Doe" },
      { id: "f2", label: "Email", type: "email", required: true, placeholder: "jane@example.com" },
    ],
    visibility: "public",
    submitLabel: "Submit",
    successMessage: "Thanks! We'll be in touch.",
  };
}

// 32-char URL-safe suffix for webhook URL secret rotation.
export function generateSecretSuffix(): string {
  const chars = "abcdefghijklmnopqrstuvwxyz0123456789";
  let out = "";
  for (let i = 0; i < 32; i++) {
    out += chars[Math.floor(Math.random() * chars.length)];
  }
  return out;
}
