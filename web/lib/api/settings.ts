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
import { hasRealSession, isDemoLive } from "@/lib/skycot/demo-mode";
import { isMissingOrUnreachable } from "./unreachable";

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

/**
 * Neutral fallback for a REAL signed-in user when `/api/v1/workspace`
 * has no row yet (e.g. a fresh signup before the workspace settings are
 * persisted). It must NOT carry the "SKYCOT Demo" branding — a real user
 * filling in their Settings should see blank fields to complete, not a
 * fabricated demo workspace name. The demo seed (`DEFAULT_WORKSPACE_SETTINGS`)
 * is reserved for anonymous demo visitors.
 */
export const NEUTRAL_WORKSPACE_SETTINGS: WorkspaceSettings = {
  name: "",
  description: "",
  default_vertical: "marketing",
  timezone: "Australia/Melbourne",
  billing_contact: "",
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
  // Per-user gate (sprint, 2026-06-07): the auth-bypass mock short-circuit
  // is for ANONYMOUS demo visitors only. A REAL signed-in user (the
  // `skycot-session=1` marker cookie is present) is NEVER bypassed — even
  // in a demo-configured deployment that sets
  // `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` — so their Settings read their
  // own real, persisted workspace instead of the "SKYCOT Demo" seed. This
  // was the stranger-signup defect: a real user saw "SKYCOT Demo" as their
  // workspace name. Mirrors the per-user gate added to `isDemoMode()` /
  // `isHireAuthBypassed()`.
  if (hasRealSession()) return false;
  // Mirrors the canonical `lib/api/skills.ts` helper exactly: a build is
  // auth-bypassed only when it is NOT a production build AND
  // `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`. The demo-live opt-out is
  // applied separately at the call sites via `isAuthBypassed() &&
  // !isDemoLive()` (see `fetchOrFallback` / `patchOrFallback`) so
  // demo-live hits the real `/api/v1/workspace`,
  // `/api/v1/providers/policy`, and `/api/v1/notifications/preferences`
  // endpoints (read-backed by the `workspace_settings` table) and
  // Settings shows real, persisted values. Plain auth-bypass dev still
  // mocks.
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

/**
 * Compute the minimal `Partial<T>` patch of `next` vs `previous` — only
 * the keys whose value actually changed. Used by the Settings panels so a
 * one-field edit PATCHes one field, not the whole object: sending the
 * whole object is a last-writer-wins clobber that overwrites concurrent
 * edits to fields the operator never touched. Shallow strict-equality is
 * correct here — every settings field is a primitive (string / number /
 * boolean).
 */
export function diffSettings<T extends object>(previous: T, next: T): Partial<T> {
  const patch: Partial<T> = {};
  for (const key of Object.keys(next) as (keyof T)[]) {
    if (next[key] !== previous[key]) {
      patch[key] = next[key];
    }
  }
  return patch;
}

async function fetchOrFallback<T>(path: string, fallback: T): Promise<{ value: T; source: SettingsSource }> {
  if (isAuthBypassed() && !isDemoLive()) return { value: fallback, source: "mock-bypass" };
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
  if (isAuthBypassed() && !isDemoLive()) return { value: merged, source: "mock-bypass" };
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
  // A real signed-in user falls back to BLANK fields (not the "SKYCOT
  // Demo" seed) if the gateway has no workspace row yet; anonymous demo
  // visitors keep the seeded showcase workspace.
  const fallback = hasRealSession()
    ? NEUTRAL_WORKSPACE_SETTINGS
    : DEFAULT_WORKSPACE_SETTINGS;
  return fetchOrFallback<WorkspaceSettings>("/api/v1/workspace", fallback);
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
