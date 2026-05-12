/**
 * Worker retrospectives API client (mission-retro-review-ui).
 *
 * The retro UI surfaces metrics aggregated from `audit_record` plus a
 * 5-section operator-fillable template (Surface metrics / What went
 * well / Surprises / What to change / Action items). Sections are
 * editable until the operator finalises the retro; the gateway PATCH
 * handler returns 409 once `finalized_at` is non-null.
 *
 * Endpoints (matches `routes_retros`):
 * - `POST   /api/v1/workers/<id>/retros` → 201 with the wire row.
 * - `PATCH  /api/v1/workers/<id>/retros/<retro_id>` → 200 with the
 *   updated wire row; 409 once finalized.
 * - `POST   /api/v1/workers/<id>/retros/<retro_id>/finalize` → 200.
 * - `GET    /api/v1/workers/<id>/retros` → list for one worker.
 * - `GET    /api/v1/retros[?q=...]` → tenant-scoped index.
 *
 * Fallback: 404 from the gateway → empty list + `mock-offline`
 * source. Mirrors `approvals.ts`. `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`
 * short-circuits with an empty mock dataset so local-dev pages work
 * without a running gateway.
 */

import { apiGet, apiPatch, apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";

export type DataSource = "live" | "mock-offline" | "mock-auth-bypass";

export const RETRO_SECTION_KEYS = [
  "surface_metrics",
  "what_went_well",
  "surprises",
  "what_to_change",
  "action_items",
] as const;

export type RetroSectionKey = (typeof RETRO_SECTION_KEYS)[number];

export type RetroSections = Record<RetroSectionKey, string>;

export interface RetroMetrics {
  actions_total: number;
  actions_approved: number;
  actions_refused: number;
  escalations: number;
  learned_skills_added: number;
  error_count_by_category: Record<string, number>;
}

export interface Retro {
  retroId: string;
  workerId: string;
  windowStart: number;
  windowEnd: number;
  metrics: RetroMetrics;
  sections: RetroSections;
  createdAt: number;
  finalizedAt: number | null;
  finalized: boolean;
}

export interface RetrosListResult {
  items: Retro[];
  source: DataSource;
}

interface RetroJson {
  retro_id: string;
  worker_id: string;
  window_start: number;
  window_end: number;
  metrics: RetroMetrics;
  sections: Partial<RetroSections>;
  created_at: number;
  finalized_at: number | null;
  finalized: boolean;
}

function authBypassEnabled(): boolean {
  return process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS === "1";
}

function emptySections(): RetroSections {
  return RETRO_SECTION_KEYS.reduce((acc, key) => {
    acc[key] = "";
    return acc;
  }, {} as RetroSections);
}

function rowToRetro(row: RetroJson): Retro {
  const sections = { ...emptySections(), ...row.sections };
  return {
    retroId: row.retro_id,
    workerId: row.worker_id,
    windowStart: row.window_start,
    windowEnd: row.window_end,
    metrics: row.metrics,
    sections,
    createdAt: row.created_at,
    finalizedAt: row.finalized_at,
    finalized: row.finalized,
  };
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

export async function fetchRetrosForWorker(
  workerId: string,
): Promise<RetrosListResult> {
  if (authBypassEnabled()) {
    return { items: [], source: "mock-auth-bypass" };
  }
  try {
    const raw = await apiGet<{ retros: RetroJson[] }>(
      `/api/v1/workers/${workerId}/retros`,
    );
    return { items: raw.retros.map(rowToRetro), source: "live" };
  } catch (err) {
    if (isMissingOrUnreachable(err)) {
      return { items: [], source: "mock-offline" };
    }
    throw err;
  }
}

export async function fetchRetrosIndex(q?: string): Promise<RetrosListResult> {
  if (authBypassEnabled()) {
    return { items: [], source: "mock-auth-bypass" };
  }
  const path = q ? `/api/v1/retros?q=${encodeURIComponent(q)}` : "/api/v1/retros";
  try {
    const raw = await apiGet<{ retros: RetroJson[] }>(path);
    return { items: raw.retros.map(rowToRetro), source: "live" };
  } catch (err) {
    if (isMissingOrUnreachable(err)) {
      return { items: [], source: "mock-offline" };
    }
    throw err;
  }
}

export interface CreateRetroInput {
  workerId: string;
  windowStartUnix?: number;
  windowEndUnix?: number;
}

export async function createRetro(input: CreateRetroInput): Promise<Retro> {
  const body: Record<string, number> = {};
  if (typeof input.windowStartUnix === "number") {
    body.window_start_unix = input.windowStartUnix;
  }
  if (typeof input.windowEndUnix === "number") {
    body.window_end_unix = input.windowEndUnix;
  }
  const row = await apiPost<RetroJson>(
    `/api/v1/workers/${input.workerId}/retros`,
    body,
  );
  return rowToRetro(row);
}

export interface UpdateRetroInput {
  workerId: string;
  retroId: string;
  sections: Partial<RetroSections>;
}

export async function updateRetroSections(
  input: UpdateRetroInput,
): Promise<Retro> {
  const row = await apiPatch<RetroJson>(
    `/api/v1/workers/${input.workerId}/retros/${input.retroId}`,
    input.sections,
  );
  return rowToRetro(row);
}

export interface FinalizeRetroInput {
  workerId: string;
  retroId: string;
}

export async function finalizeRetro(input: FinalizeRetroInput): Promise<Retro> {
  const row = await apiPost<RetroJson>(
    `/api/v1/workers/${input.workerId}/retros/${input.retroId}/finalize`,
    {},
  );
  return rowToRetro(row);
}

/** Operator-readable label for one section key. Single source so the
 *  dialog stepper and the index theme search stay aligned. */
export const RETRO_SECTION_LABELS: Record<RetroSectionKey, string> = {
  surface_metrics: "Surface metrics",
  what_went_well: "What went well",
  surprises: "Surprises",
  what_to_change: "What to change",
  action_items: "Action items",
};

/** Helper for the dialog's metric-summary step. */
export function summarizeMetricsLine(metrics: RetroMetrics): string {
  const parts = [
    `${metrics.actions_total} actions`,
    `${metrics.actions_approved} approved`,
    `${metrics.actions_refused} refused`,
  ];
  if (metrics.escalations > 0) parts.push(`${metrics.escalations} escalations`);
  if (metrics.learned_skills_added > 0) {
    parts.push(`${metrics.learned_skills_added} skills learned`);
  }
  return parts.join(" • ");
}
