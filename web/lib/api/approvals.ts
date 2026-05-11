/**
 * Approvals API client.
 *
 * The gateway currently serves `/inbox` as HTML (see crates/argo-cli/
 * src/handlers/gateway/routes_channels_webinbox.rs) — not yet a JSON
 * endpoint. Until a `/api/v1/approvals` JSON route lands (tracked in
 * MISSION_BACKLOG.md #123), the `fetchApprovals` function below
 * returns the mock dataset.
 *
 * When the JSON endpoint is ready, swap the body of `fetchApprovals`
 * for a real fetch — TanStack Query consumers don't change.
 */

import { SAMPLE_APPROVALS, type ApprovalItem } from "@/lib/sample-approvals";

const GATEWAY_URL =
  process.env.NEXT_PUBLIC_GATEWAY_URL ?? "http://127.0.0.1:8787";

export type ApprovalDecisionRequest = {
  approvalId: string;
  decision: "approve" | "refuse";
};

/**
 * Fetch the approval inbox. **Currently returns mock data** because
 * the gateway's `/inbox` is HTML, not JSON. Real wire-up is one line
 * below the TODO.
 */
export async function fetchApprovals(): Promise<ApprovalItem[]> {
  // Simulate the gateway-latency floor so loading states render.
  await new Promise((r) => setTimeout(r, 120));
  return SAMPLE_APPROVALS;

  // TODO(track-e): real wire-up once a JSON inbox lands. Approximate
  // shape:
  // const res = await fetch(`${GATEWAY_URL}/api/v1/approvals`, {
  //   headers: { Authorization: `Bearer ${getBearerToken()}` },
  //   credentials: "include",
  // });
  // if (!res.ok) throw Object.assign(new Error(res.statusText), { status: res.status });
  // return await res.json();
}

/**
 * Submit a decision. Today this is a no-op + 80ms delay so the
 * optimistic-update flow can be exercised end-to-end. Real wire-up
 * (also TODO(track-e)) POSTs to `/inbox/approve/<id>` or
 * `/inbox/reject/<id>` per the existing gateway shape.
 */
export async function submitDecision({
  approvalId,
  decision,
}: ApprovalDecisionRequest): Promise<{ ok: true; approvalId: string; decision: "approve" | "refuse" }> {
  await new Promise((r) => setTimeout(r, 80));
  return { ok: true, approvalId, decision };

  // TODO(track-e):
  // const path = decision === "approve"
  //   ? `/inbox/approve/${approvalId}`
  //   : `/inbox/reject/${approvalId}`;
  // const res = await fetch(`${GATEWAY_URL}${path}`, {
  //   method: "POST",
  //   headers: { Authorization: `Bearer ${getBearerToken()}` },
  //   credentials: "include",
  // });
  // if (!res.ok) throw Object.assign(new Error(res.statusText), { status: res.status });
  // return await res.json();
}

/** Unused today (mock has no undo) — real wire-up will POST `/inbox/undo/<id>`. */
export async function submitUndo(approvalId: string): Promise<{ ok: true; approvalId: string }> {
  await new Promise((r) => setTimeout(r, 60));
  return { ok: true, approvalId };
}

// Suppress unused-warning until we wire to the real endpoint.
void GATEWAY_URL;
