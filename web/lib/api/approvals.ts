/**
 * Approvals API client.
 *
 * Routes through the typed `lib/api/client.ts` wrapper. The gateway's
 * JSON inbox endpoint is being authored under brief
 * `J1-gateway-approvals-json-api`; until that lands, every call here
 * will see a 404 and fall back to the bundled mock dataset so the
 * approvals demo keeps working in dev without a running gateway.
 *
 * Acceptance contract:
 * - `fetchApprovals` returns `ApprovalItem[]`.
 * - `submitDecision` / `submitUndo` return success envelopes.
 * - On 404 from the gateway, fall back to mock data (preserves the
 *   dev demo). Other ApiError statuses propagate.
 * - On NetworkError (gateway unreachable) the helpers fall back to
 *   the mock dataset as well — the dev story is "you don't need a
 *   gateway running to see the inbox."
 */

import { apiGet, apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { SAMPLE_APPROVALS, type ApprovalItem } from "@/lib/sample-approvals";

export type ApprovalDecisionRequest = {
  approvalId: string;
  decision: "approve" | "refuse";
};

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

export async function fetchApprovals(): Promise<ApprovalItem[]> {
  try {
    return await apiGet<ApprovalItem[]>("/api/v1/approvals");
  } catch (err) {
    if (isMissingOrUnreachable(err)) {
      await new Promise((r) => setTimeout(r, 120));
      return SAMPLE_APPROVALS;
    }
    throw err;
  }
}

export async function submitDecision({
  approvalId,
  decision,
}: ApprovalDecisionRequest): Promise<{ ok: true; approvalId: string; decision: "approve" | "refuse" }> {
  const path =
    decision === "approve"
      ? `/api/v1/approvals/${approvalId}/approve`
      : `/api/v1/approvals/${approvalId}/refuse`;
  try {
    return await apiPost<{ ok: true; approvalId: string; decision: "approve" | "refuse" }>(path);
  } catch (err) {
    if (isMissingOrUnreachable(err)) {
      await new Promise((r) => setTimeout(r, 80));
      return { ok: true, approvalId, decision };
    }
    throw err;
  }
}

export async function submitUndo(approvalId: string): Promise<{ ok: true; approvalId: string }> {
  try {
    return await apiPost<{ ok: true; approvalId: string }>(
      `/api/v1/approvals/${approvalId}/undo`,
    );
  } catch (err) {
    if (isMissingOrUnreachable(err)) {
      await new Promise((r) => setTimeout(r, 60));
      return { ok: true, approvalId };
    }
    throw err;
  }
}
