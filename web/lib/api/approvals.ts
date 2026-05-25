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
 * - `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` → skip all fetches, return
 *   mock, `source = "mock-auth-bypass"`.
 * - Any other `ApiError` (4xx/5xx) propagates and surfaces as the
 *   inbox's error state.
 */

import { apiGet, apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import type { Severity } from "@/components/skycot/approvals/SeverityPill";
import {
  SAMPLE_APPROVALS,
  type ApprovalItem,
  type ChannelKind,
} from "@/lib/sample-approvals";
import { isDemoMode } from "@/lib/skycot/demo-mode";

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
  return process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1";
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

function isForbiddenError(err: unknown): err is ApiError {
  return err instanceof ApiError && err.status === 403;
}

/**
 * Surface the gateway's role-gate denial (403) as a sonner toast.
 *
 * The gateway role-gate (mission-rbac-approvals-decide-role-gate)
 * returns 403 for any caller below the `user` floor (or below `owner`
 * for some sister actions). The previous code returned `ok: true`
 * silently when the network "looked unreachable", which masked role
 * denials in dev/preview and corrupted the audit trail. We now show
 * the operator the real reason — they need an `admin` or `owner` to
 * intervene, the action is NOT recorded as decided.
 *
 * Sonner is a browser-only singleton; calling it from a server
 * component would throw. The dynamic import + browser-env guard keeps
 * the function safe to call from any layer.
 */
function toastForbidden(scope: "approve" | "undo"): void {
  if (typeof window === "undefined") return;
  const title =
    scope === "approve"
      ? "You don't have permission to approve"
      : "You don't have permission to undo this decision";
  const description =
    scope === "approve"
      ? "Only owners, admins, and approvers can decide approvals. Ask an admin to grant you the role."
      : "Only owners, admins, and approvers can undo within 30 seconds.";
  // Sonner's `toast` is a static singleton imported lazily so SSR
  // bundles can dead-code the toast surface.
  void import("sonner")
    .then((m) => m.toast.error(title, { description }))
    .catch(() => {
      /* sonner not installed / failed to load — silent fallback */
    });
}

type ApprovalRowJson = {
  id: string;
  action_id?: string | null;
  worker: { id?: string; name: string; avatar_url?: string | null };
  verb: string;
  detail: string;
  consequence_if_yes: string;
  consequence_if_no: string;
  reason?: string | null;
  draft_content?: string | null;
  channel: ChannelKind;
  received_at: string;
  decision?: "approve" | "refuse" | null;

  // Ops-dashboard fields. All optional so older gateway envelopes
  // (pre this brief) keep deserialising; the UI falls back to default
  // severity "p2" and hides the $-at-risk / confidence / SLA chrome
  // when the row omits them.
  severity?: Severity | null;
  dollar_at_risk?: number | null;
  currency?: string | null;
  confidence?: number | null;
  sla_expires_at?: string | null;
};

type ApprovalsListJson = {
  items: ApprovalRowJson[];
  next_cursor?: string | null;
};

type ApprovalItemWithActionId = ApprovalItem & { actionId?: string };

function rowToItem(row: ApprovalRowJson): ApprovalItem {
  const item: ApprovalItemWithActionId = {
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
    draftContent: row.draft_content ?? undefined,
    channel: row.channel,
    receivedAt: row.received_at,
    decision: row.decision ?? null,
    severity: row.severity ?? undefined,
    dollarAtRisk: row.dollar_at_risk ?? undefined,
    currency: row.currency ?? undefined,
    confidence: row.confidence ?? undefined,
    slaExpiresAt: row.sla_expires_at ?? undefined,
  };
  if (row.action_id) item.actionId = row.action_id;
  return item;
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
      // Demo mode keeps the seeded SAMPLE_APPROVALS so the v1.0
      // recorded walkthrough still plays back. Real signed-in users on
      // a fresh workspace fall through to the empty inbox state.
      if (isDemoMode()) {
        return { items: SAMPLE_APPROVALS, source: "mock-offline" };
      }
      return { items: [], source: "live" };
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
    // RBAC role-gate (mission-rbac-approvals-decide-role-gate): a
    // 403 from the gateway means the caller's role is below the
    // `user` floor for `/decide`. Surface a toast so the operator
    // sees the real reason — the previous mock-offline fallback
    // masked this as a silent success and corrupted the audit trail.
    if (isForbiddenError(err)) {
      toastForbidden("approve");
      throw err;
    }
    // LAUNCH-BLOCKER FIX (Wave-8 error-recovery audit 2026-05-21):
    // previous code returned `ok: true, source: "mock-offline"` on
    // ANY network failure including production. Result: user clicks
    // Approve, sees the green tick, BUT THE GATEWAY NEVER RECORDED
    // THE DECISION. Audit-trail integrity hole — every "approve"
    // we fail to send is missing from the chain. In strict prod we
    // now propagate the error so the optimistic mutation rolls back
    // + user sees the error toast. Dev/preview keep the mock
    // fallback so demos with no gateway still work.
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
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
    if (isForbiddenError(err)) {
      toastForbidden("undo");
      throw err;
    }
    // Same launch-blocker fix as submitDecision — never silently
    // succeed in strict production on a gateway-unreachable error.
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return { ok: true, approvalId, source: "mock-offline" };
    }
    throw err;
  }
}

function isStrictProduction(): boolean {
  return (
    process.env.NODE_ENV === "production" &&
    process.env.NEXT_PUBLIC_ENV === "production"
  );
}
