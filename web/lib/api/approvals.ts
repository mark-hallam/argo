/**
 * Approvals API client.
 *
 * Returns data alongside a `source` discriminator so the UI can
 * surface whether the inbox is reading the real gateway (`live`),
 * falling back to the bundled mock dataset because the gateway
 * is unreachable or hasn't shipped the JSON endpoint yet
 * (`mock-offline`), or short-circuiting in local-dev auth-bypass
 * mode (`mock-auth-bypass`).
 *
 * Endpoint contract (matches `J1-gateway-approvals-json-api`):
 * - `GET /api/v1/approvals` → `{ items: ApprovalRow[], next_cursor? }`
 *   where `ApprovalRow` is the snake_case server shape. We map to
 *   the camelCase `ApprovalItem` consumed by the UI.
 * - `POST /api/v1/approvals/<id>/decide` body `{ decision }` →
 *   `{ ok, approval_id, decision }`.
 * - `POST /api/v1/approvals/<id>/undo` → `{ ok, approval_id }`.
 *
 * Fallback behaviour:
 * - 404 from the gateway (J1 not deployed yet) → return mock,
 *   `source = "mock-offline"`.
 * - `TypeError` from `fetch` (connection refused / DNS failure) →
 *   same fallback.
 * - `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` → skip all fetches, return
 *   mock, `source = "mock-auth-bypass"`.
 * - Any other `ApiError` (4xx/5xx) propagates and surfaces as the
 *   inbox's error state.
 */

import { apiGet, apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import {
  SAMPLE_APPROVALS,
  type ApprovalItem,
  type ChannelKind,
} from "@/lib/sample-approvals";

export type DataSource = "live" | "mock-offline" | "mock-auth-bypass";

export type ApprovalsResult = {
  items: ApprovalItem[];
  source: DataSource;
};

export type ApprovalDecisionRequest = {
  approvalId: string;
  decision: "approve" | "refuse";
};

export type DecisionResult = {
  ok: true;
  approvalId: string;
  decision: "approve" | "refuse";
  source: DataSource;
};

export type UndoResult = {
  ok: true;
  approvalId: string;
  source: DataSource;
};

/**
 * `NEXT_PUBLIC_*` env vars are inlined at build time, same model as
 * `NEXT_PUBLIC_GATEWAY_URL` in `client.ts`. Reading at module load
 * rather than per-call lets the bundler dead-code the gateway path
 * in pure-local-dev bundles.
 *
 * Exported as a function so unit tests can re-evaluate after
 * mutating `process.env`.
 */
export function authBypassEnabled(): boolean {
  return process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS === "1";
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

type ApprovalRowJson = {
  id: string;
  worker: { id?: string; name: string; avatar_url?: string | null };
  verb: string;
  detail: string;
  consequence_if_yes: string;
  consequence_if_no: string;
  reason?: string | null;
  channel: ChannelKind;
  received_at: string;
  decision?: "approve" | "refuse" | null;
};

type ApprovalsListJson = {
  items: ApprovalRowJson[];
  next_cursor?: string | null;
};

function rowToItem(row: ApprovalRowJson): ApprovalItem {
  return {
    id: row.id,
    worker: {
      name: row.worker.name,
      avatarUrl: row.worker.avatar_url ?? undefined,
    },
    verb: row.verb,
    detail: row.detail,
    consequenceIfYes: row.consequence_if_yes,
    consequenceIfNo: row.consequence_if_no,
    reason: row.reason ?? undefined,
    channel: row.channel,
    receivedAt: row.received_at,
    decision: row.decision ?? null,
  };
}

export async function fetchApprovals(): Promise<ApprovalsResult> {
  if (authBypassEnabled()) {
    return { items: SAMPLE_APPROVALS, source: "mock-auth-bypass" };
  }
  try {
    const raw = await apiGet<ApprovalsListJson | ApprovalRowJson[]>(
      "/api/v1/approvals",
    );
    const rows = Array.isArray(raw) ? raw : raw.items;
    return { items: rows.map(rowToItem), source: "live" };
  } catch (err) {
    if (isMissingOrUnreachable(err)) {
      return { items: SAMPLE_APPROVALS, source: "mock-offline" };
    }
    throw err;
  }
}

export async function submitDecision({
  approvalId,
  decision,
}: ApprovalDecisionRequest): Promise<DecisionResult> {
  if (authBypassEnabled()) {
    return { ok: true, approvalId, decision, source: "mock-auth-bypass" };
  }
  try {
    const res = await apiPost<{
      ok: true;
      approval_id: string;
      decision: "approve" | "refuse";
    }>(`/api/v1/approvals/${approvalId}/decide`, { decision });
    return {
      ok: true,
      approvalId: res.approval_id ?? approvalId,
      decision: res.decision ?? decision,
      source: "live",
    };
  } catch (err) {
    if (isMissingOrUnreachable(err)) {
      return { ok: true, approvalId, decision, source: "mock-offline" };
    }
    throw err;
  }
}

export async function submitUndo(approvalId: string): Promise<UndoResult> {
  if (authBypassEnabled()) {
    return { ok: true, approvalId, source: "mock-auth-bypass" };
  }
  try {
    const res = await apiPost<{ ok: true; approval_id: string }>(
      `/api/v1/approvals/${approvalId}/undo`,
    );
    return { ok: true, approvalId: res.approval_id ?? approvalId, source: "live" };
  } catch (err) {
    if (isMissingOrUnreachable(err)) {
      return { ok: true, approvalId, source: "mock-offline" };
    }
    throw err;
  }
}
