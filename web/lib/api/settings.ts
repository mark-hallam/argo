/**
 * Workspace-level settings client — `workspace`, `providers`,
 * `notifications`.
 *
 * Backs the three Settings sub-pages that the F13 audit flagged as
 * read-only / hardcoded. Each panel has a single shape:
 *
 *   - `GET    /api/v1/workspace`                      → WorkspaceSettings
 *   - `PATCH  /api/v1/workspace`                      ← Partial<WorkspaceSettings>
 *   - `GET    /api/v1/providers/policy`               → ProviderSettings
 *   - `PATCH  /api/v1/providers/policy`               ← Partial<ProviderSettings>
 *   - `GET    /api/v1/notifications/preferences`      → NotificationSettings
 *   - `PATCH  /api/v1/notifications/preferences`      ← Partial<NotificationSettings>
 *
 * Auth-bypass / 404 fallback contract matches `lib/api/api-keys.ts`:
 *
 *   - `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` → bundled defaults,
 *     mutations are no-ops that round-trip the patched body.
 *   - 404 / NetworkError in dev → bundled defaults.
 *   - Strict production propagates all errors.
 */

import { apiGet, apiPatch } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";

export interface WorkspaceSettings {
  name: string;
  description: string;
  default_vertical: string;
  timezone: string;
  billing_contact: string;
}

export const VERTICAL_OPTIONS: readonly { value: string; label: string }[] = [
  { value: "marketing", label: "Marketing operations" },
  { value: "sales", label: "Sales operations" },
  { value: "support", label: "Customer support" },
  { value: "ops", label: "Workspace operations" },
  { value: "research", label: "Research" },
] as const;

export const TIMEZONE_OPTIONS: readonly string[] = [
  "Australia/Melbourne",
  "Australia/Sydney",
  "America/Los_Angeles",
  "America/New_York",
  "Europe/London",
  "Asia/Singapore",
  "UTC",
] as const;

export const DEFAULT_WORKSPACE_SETTINGS: WorkspaceSettings = {
  name: "SKYCOT Demo",
  description: "Operator workspace for the SKYCOT demo tenant.",
  default_vertical: "marketing",
  timezone: "Australia/Melbourne",
  billing_contact: "owner@skycot.example",
};

export type ProviderName = "anthropic" | "openai" | "google" | "azure" | "bedrock";

export const PROVIDER_OPTIONS: readonly { value: ProviderName; label: string }[] = [
  { value: "anthropic", label: "Anthropic" },
  { value: "openai", label: "OpenAI" },
  { value: "google", label: "Google" },
  { value: "azure", label: "Azure OpenAI" },
  { value: "bedrock", label: "AWS Bedrock" },
] as const;

export type FallbackBehavior = "fail_closed" | "fail_open" | "retry_alt";

export const FALLBACK_OPTIONS: readonly { value: FallbackBehavior; label: string }[] = [
  { value: "fail_closed", label: "Fail closed — stop and audit" },
  { value: "fail_open", label: "Fail open — proceed with alternates" },
  { value: "retry_alt", label: "Retry on alternates, then audit" },
] as const;

export interface ProviderSettings {
  primary_provider: ProviderName;
  fallback_behavior: FallbackBehavior;
  allow_worker_overrides: boolean;
}

export const DEFAULT_PROVIDER_SETTINGS: ProviderSettings = {
  primary_provider: "anthropic",
  fallback_behavior: "fail_closed",
  allow_worker_overrides: true,
};

export interface NotificationSettings {
  approval_escalation_minutes: number;
  daily_digest_time: string;
  quiet_hours_start: string;
  quiet_hours_end: string;
  email_enabled: boolean;
  push_enabled: boolean;
}

export const DEFAULT_NOTIFICATION_SETTINGS: NotificationSettings = {
  approval_escalation_minutes: 60,
  daily_digest_time: "08:30",
  quiet_hours_start: "18:00",
  quiet_hours_end: "08:00",
  email_enabled: true,
  push_enabled: false,
};

export type SettingsSource = "gateway" | "mock-offline" | "mock-bypass";

function isStrictProduction(): boolean {
  return (
    process.env.NODE_ENV === "production" &&
    process.env.NEXT_PUBLIC_ENV === "production"
  );
}

function isAuthBypassed(): boolean {
  // Recording-mode opt-out (`NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1`): demo-live
  // hits the real `/api/v1/workspace`, `/api/v1/providers/policy`, and
  // `/api/v1/notifications/preferences` endpoints (read-backed by the
  // `workspace_settings` table) so Settings shows real, persisted
  // values. Equivalent to the canonical `!isDemoLive()` guard used
  // across the other api modules. Plain auth-bypass dev still mocks.
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1" &&
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE !== "1"
  );
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

async function fetchOrFallback<T>(path: string, fallback: T): Promise<{ value: T; source: SettingsSource }> {
  if (isAuthBypassed()) return { value: fallback, source: "mock-bypass" };
  try {
    const value = await apiGet<T>(path);
    return { value, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return { value: fallback, source: "mock-offline" };
    }
    throw err;
  }
}

async function patchOrFallback<T>(
  path: string,
  previous: T,
  patch: Partial<T>,
): Promise<{ value: T; source: SettingsSource }> {
  const merged = { ...previous, ...patch };
  if (isAuthBypassed()) return { value: merged, source: "mock-bypass" };
  try {
    const value = await apiPatch<T, Partial<T>>(path, patch);
    return { value, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return { value: merged, source: "mock-offline" };
    }
    throw err;
  }
}

export async function fetchWorkspaceSettings() {
  return fetchOrFallback<WorkspaceSettings>("/api/v1/workspace", DEFAULT_WORKSPACE_SETTINGS);
}

export async function updateWorkspaceSettings(
  previous: WorkspaceSettings,
  patch: Partial<WorkspaceSettings>,
) {
  return patchOrFallback<WorkspaceSettings>("/api/v1/workspace", previous, patch);
}

export async function fetchProviderSettings() {
  return fetchOrFallback<ProviderSettings>("/api/v1/providers/policy", DEFAULT_PROVIDER_SETTINGS);
}

export async function updateProviderSettings(
  previous: ProviderSettings,
  patch: Partial<ProviderSettings>,
) {
  return patchOrFallback<ProviderSettings>("/api/v1/providers/policy", previous, patch);
}

export async function fetchNotificationSettings() {
  return fetchOrFallback<NotificationSettings>(
    "/api/v1/notifications/preferences",
    DEFAULT_NOTIFICATION_SETTINGS,
  );
}

export async function updateNotificationSettings(
  previous: NotificationSettings,
  patch: Partial<NotificationSettings>,
) {
  return patchOrFallback<NotificationSettings>(
    "/api/v1/notifications/preferences",
    previous,
    patch,
  );
}
