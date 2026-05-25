/**
 * Worker-settings API client.
 *
 * Backs the six-section Settings tab — Channels / Quiet hours /
 * Budget / Autonomy / Advanced / Danger zone — with the same
 * mock-fallback contract as `worker-skills.ts` and
 * `worker-activity.ts`:
 *
 *   - `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` → mock immediately.
 *   - 404 / network error in dev → bundled mock.
 *   - Strict production (`NODE_ENV === 'production'` AND
 *     `NEXT_PUBLIC_ENV === 'production'`) propagates all errors.
 *
 * The gateway endpoints arrive via the sibling brief
 * `mission-gateway-worker-settings-endpoints`. Until those land,
 * every read is satisfied locally and mutations are no-ops on the
 * server but produce a structured payload the hook can echo back to
 * the cache for optimistic UI.
 */

import { apiDelete, apiGet, apiPatch, apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";

export type ChannelKind =
  | "slack"
  | "imessage"
  | "sms"
  | "email"
  | "push"
  | "whatsapp"
  | "voice";

export const CHANNEL_KINDS: readonly ChannelKind[] = [
  "slack",
  "imessage",
  "sms",
  "email",
  "push",
  "whatsapp",
  "voice",
] as const;

export const CHANNEL_LABELS: Record<ChannelKind, string> = {
  slack: "Slack",
  imessage: "iMessage",
  sms: "SMS",
  email: "Email",
  push: "Push",
  whatsapp: "WhatsApp",
  voice: "Voice",
};

export type AutonomyLevel = "conservative" | "balanced" | "aggressive";

export const AUTONOMY_LEVELS: readonly AutonomyLevel[] = [
  "conservative",
  "balanced",
  "aggressive",
] as const;

export const AUTONOMY_LABELS: Record<AutonomyLevel, string> = {
  conservative: "Conservative — ask often",
  balanced: "Balanced — current",
  aggressive: "Aggressive — minimal asking",
};

export interface ChannelsConfig {
  enabled: ChannelKind[];
  primary: ChannelKind;
}

export interface QuietHoursConfig {
  /** 24h "HH:mm" — start of the quiet window. */
  start: string;
  /** 24h "HH:mm" — end of the quiet window. */
  end: string;
  /** IANA tz id, e.g. "America/Los_Angeles". */
  timezone: string;
}

export interface BudgetConfig {
  /** Daily cap in whole dollars. */
  dailyCapUsd: number;
  /** Monthly cap in whole dollars. */
  monthlyCapUsd: number;
  /** Spend today (mock surfaces a realistic number; gateway returns live). */
  usedTodayUsd: number;
  /** Spend month-to-date. */
  usedMonthUsd: number;
}

export interface AutonomyConfig {
  level: AutonomyLevel;
}

export interface AdvancedConfig {
  /** Worker manifest as a single TOML string for the read-only viewer. */
  manifestToml: string;
  /** Workspace, tenant, namespace ids surfaced in the provenance chip. */
  tenantId: string;
  namespaceId: string;
  versionHash: string;
}

export interface WorkerSettings {
  channels: ChannelsConfig;
  quietHours: QuietHoursConfig;
  budget: BudgetConfig;
  autonomy: AutonomyConfig;
  advanced: AdvancedConfig;
}

export type WorkerSettingsSource = "gateway" | "mock-offline" | "mock-bypass";

export interface FetchWorkerSettingsResult {
  settings: WorkerSettings;
  source: WorkerSettingsSource;
}

export class WorkerSettingsNotFoundError extends Error {
  readonly workerId: string;
  constructor(workerId: string) {
    super(`Worker settings for '${workerId}' not found`);
    this.name = "WorkerSettingsNotFoundError";
    this.workerId = workerId;
  }
}

export interface SettingsMutationResult {
  source: WorkerSettingsSource;
}

function isStrictProduction(): boolean {
  return (
    process.env.NODE_ENV === "production" &&
    process.env.NEXT_PUBLIC_ENV === "production"
  );
}

function isAuthBypassed(): boolean {
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

function detectTimezone(): string {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
  } catch {
    return "UTC";
  }
}

function mockSettings(workerId: string): WorkerSettings {
  const tz = detectTimezone();
  return {
    channels: {
      enabled: ["slack", "imessage", "email"],
      primary: "slack",
    },
    quietHours: {
      start: "21:00",
      end: "08:00",
      timezone: tz,
    },
    budget: {
      dailyCapUsd: 5,
      monthlyCapUsd: 75,
      usedTodayUsd: 4.32,
      usedMonthUsd: 41.6,
    },
    autonomy: { level: "balanced" },
    advanced: {
      manifestToml: sampleManifestToml(workerId),
      tenantId: "tnt_workspace",
      namespaceId: "ns_default",
      versionHash: "v0.1.0+sha.4b3c2a1",
    },
  };
}

function sampleManifestToml(workerId: string): string {
  return [
    `# Role manifest for worker ${workerId}`,
    `version = "0.1.0"`,
    `vertical = "marketplace"`,
    ``,
    `[charter]`,
    `summary = "Lists and re-prices items for sale"`,
    ``,
    `[duties]`,
    `enabled = ["draft_listings", "reprice_stale"]`,
    ``,
    `[autonomy]`,
    `level = "balanced"`,
    ``,
    `[budget]`,
    `daily_cap_usd = 5`,
    `monthly_cap_usd = 75`,
  ].join("\n");
}

function mockResult(workerId: string, source: WorkerSettingsSource): FetchWorkerSettingsResult {
  return { settings: mockSettings(workerId), source };
}

export async function getSettings(workerId: string): Promise<FetchWorkerSettingsResult> {
  if (isAuthBypassed()) return mockResult(workerId, "mock-bypass");

  try {
    const raw = await apiGet<unknown>(
      `/api/v1/workers/${encodeURIComponent(workerId)}/settings`,
    );
    const settings = parseGatewaySettings(raw, workerId);
    return { settings, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockResult(workerId, "mock-offline");
    }
    if (err instanceof ApiError && err.status === 404) {
      throw new WorkerSettingsNotFoundError(workerId);
    }
    throw err;
  }
}

/**
 * Map the gateway's snake-case settings envelope into the TS camelCase
 * `WorkerSettings` shape. The two surfaces diverged historically — the
 * gateway emits `{ channels: { all, primary }, quiet_hours: {...,tz},
 * budget: { daily_usd, ... }, advanced: { namespace_id, manifest_url, ...} }`
 * and the page components consume `{ channels: { enabled, primary },
 * quietHours: { ..., timezone }, budget: { dailyCapUsd, ... } }`. Doing
 * the mapping here keeps the gateway tests stable while letting the
 * page render persisted values. Missing fields fall back to the mock
 * defaults so the page never crashes on a partial payload.
 */
function parseGatewaySettings(raw: unknown, workerId: string): WorkerSettings {
  const fallback = mockSettings(workerId);
  if (!raw || typeof raw !== "object") return fallback;
  const r = raw as Record<string, unknown>;

  const channels = asRecord(r.channels);
  const all = Array.isArray(channels?.all)
    ? (channels.all as unknown[]).filter(isChannelKind)
    : Array.isArray(channels?.enabled)
      ? (channels.enabled as unknown[]).filter(isChannelKind)
      : fallback.channels.enabled;
  const primaryRaw = channels?.primary;
  const primary = isChannelKind(primaryRaw) ? primaryRaw : (all[0] ?? fallback.channels.primary);

  const quiet = asRecord(r.quiet_hours) ?? asRecord(r.quietHours);
  const tz =
    asString(quiet?.tz) ?? asString(quiet?.timezone) ?? fallback.quietHours.timezone;

  const budget = asRecord(r.budget);
  const daily =
    asNumber(budget?.daily_usd) ?? asNumber(budget?.dailyCapUsd) ?? fallback.budget.dailyCapUsd;
  const monthly =
    asNumber(budget?.monthly_usd) ??
    asNumber(budget?.monthlyCapUsd) ??
    fallback.budget.monthlyCapUsd;
  const usedToday =
    asNumber(budget?.used_today_usd) ??
    asNumber(budget?.usedTodayUsd) ??
    fallback.budget.usedTodayUsd;
  const usedMonth =
    asNumber(budget?.used_month_usd) ??
    asNumber(budget?.usedMonthUsd) ??
    fallback.budget.usedMonthUsd;

  const autonomy = asRecord(r.autonomy);
  const level = asString(autonomy?.level);
  const levelTyped: AutonomyLevel = AUTONOMY_LEVELS.includes(level as AutonomyLevel)
    ? (level as AutonomyLevel)
    : fallback.autonomy.level;

  const advanced = asRecord(r.advanced);
  const advancedConfig: AdvancedConfig = advanced
    ? {
        manifestToml:
          asString(advanced.manifestToml) ??
          asString(advanced.manifest_toml) ??
          fallback.advanced.manifestToml,
        tenantId:
          asString(advanced.tenantId) ??
          asString(advanced.tenant_id) ??
          fallback.advanced.tenantId,
        namespaceId:
          asString(advanced.namespaceId) ??
          asString(advanced.namespace_id) ??
          fallback.advanced.namespaceId,
        versionHash:
          asString(advanced.versionHash) ??
          asString(advanced.version_hash) ??
          fallback.advanced.versionHash,
      }
    : fallback.advanced;

  return {
    channels: { enabled: all, primary },
    quietHours: {
      start: asString(quiet?.start) ?? fallback.quietHours.start,
      end: asString(quiet?.end) ?? fallback.quietHours.end,
      timezone: tz,
    },
    budget: {
      dailyCapUsd: daily,
      monthlyCapUsd: monthly,
      usedTodayUsd: usedToday,
      usedMonthUsd: usedMonth,
    },
    autonomy: { level: levelTyped },
    advanced: advancedConfig,
  };
}

function asRecord(v: unknown): Record<string, unknown> | undefined {
  return v && typeof v === "object" && !Array.isArray(v)
    ? (v as Record<string, unknown>)
    : undefined;
}

function asString(v: unknown): string | undefined {
  return typeof v === "string" && v.length > 0 ? v : undefined;
}

function asNumber(v: unknown): number | undefined {
  return typeof v === "number" && Number.isFinite(v) ? v : undefined;
}

function isChannelKind(v: unknown): v is ChannelKind {
  return typeof v === "string" && (CHANNEL_KINDS as readonly string[]).includes(v);
}

async function mutate(
  workerId: string,
  path: string,
  method: "patch" | "post" | "delete",
  body?: unknown,
): Promise<WorkerSettingsSource> {
  if (isAuthBypassed()) return "mock-bypass";
  const url = `/api/v1/workers/${encodeURIComponent(workerId)}${path}`;
  try {
    if (method === "patch") {
      await apiPatch<unknown>(url, body);
    } else if (method === "post") {
      await apiPost<unknown>(url, body);
    } else {
      await apiDelete<unknown>(url);
    }
    return "gateway";
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return "mock-offline";
    }
    throw err;
  }
}

export async function updateChannels(
  workerId: string,
  channels: ChannelsConfig,
): Promise<SettingsMutationResult> {
  const source = await mutate(workerId, `/settings/channels`, "patch", channels);
  return { source };
}

export async function updateQuietHours(
  workerId: string,
  quietHours: QuietHoursConfig,
): Promise<SettingsMutationResult> {
  const source = await mutate(workerId, `/settings/quiet-hours`, "patch", quietHours);
  return { source };
}

export async function updateBudget(
  workerId: string,
  budget: Pick<BudgetConfig, "dailyCapUsd" | "monthlyCapUsd">,
): Promise<SettingsMutationResult> {
  const source = await mutate(workerId, `/settings/budget`, "patch", budget);
  return { source };
}

export async function updateAutonomy(
  workerId: string,
  autonomy: AutonomyConfig,
): Promise<SettingsMutationResult> {
  const source = await mutate(workerId, `/settings/autonomy`, "patch", autonomy);
  return { source };
}

export async function pauseWorker(workerId: string): Promise<SettingsMutationResult> {
  const source = await mutate(workerId, `/pause`, "post", {});
  return { source };
}

export async function archiveWorker(workerId: string): Promise<SettingsMutationResult> {
  const source = await mutate(workerId, `/archive`, "post", {});
  return { source };
}

export async function deleteWorker(workerId: string): Promise<SettingsMutationResult> {
  const source = await mutate(workerId, ``, "delete");
  return { source };
}

/** Budget usage bucket → progress-bar tone. */
export type BudgetTone = "muted" | "default" | "warning" | "destructive";

export function budgetTone(percent: number): BudgetTone {
  if (percent >= 95) return "destructive";
  if (percent >= 80) return "warning";
  if (percent >= 50) return "default";
  return "muted";
}

/** Percentage 0..100 of used vs cap; clamped, never NaN. */
export function budgetPercent(used: number, cap: number): number {
  if (!Number.isFinite(used) || !Number.isFinite(cap) || cap <= 0) return 0;
  const raw = (used / cap) * 100;
  if (raw < 0) return 0;
  if (raw > 100) return 100;
  return raw;
}
