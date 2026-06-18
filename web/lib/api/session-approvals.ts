/**
 * Session tool-approval API client (day-1 unblock FIX 1).
 *
 * A fresh teammate runs inside a 24-hour probation window: every tool
 * needs a one-time human OK before it can fire. The gateway exposes
 * the grant flow at the BARE `POST /sessions/<id>/approve` (#665); this
 * module reaches it via the shared authenticated `apiPost` client at
 * `/api/v1/sessions/<id>/approve`, which `next.config.ts` rewrites to the
 * gateway's `/sessions/*`. `apiPost` stamps the `__Host-session` cookie +
 * `X-Argo-Principal` / `X-Argo-Tenant-Id` + CSRF token — the same proven
 * path hire / run-now use. (The old `app/api/sessions/[id]/approve` Next
 * proxy relied on a server-side NextAuth `session.gatewayToken` that is
 * empty under the production magic-link flow — the "Approve & run" 401.)
 *
 * Body shape (one tool per call, matching the gateway contract):
 *   `{ tool: string, note?: string, one_shot?: boolean }`
 * Success envelope: `{ ok, grant_id, session_id, tool, one_shot }`.
 *
 * `approveSessionTools` fans a held-tools list out as sequential
 * per-tool grants and fails fast on the first error so the caller can
 * surface the gateway's message (any already-granted tools keep their
 * grants — re-approving is idempotent-enough: a duplicate grant is
 * harmless and audit-visible).
 */

import { apiPost } from "@/lib/api/client";

export interface SessionToolGrant {
  ok: boolean;
  grant_id: string;
  session_id: string;
  tool: string;
  one_shot: boolean;
}

export interface ApproveSessionToolOptions {
  /** Free-text note recorded on the grant's audit row. */
  note?: string;
  /**
   * `true` = the grant is consumed by the first matching tool call
   * (the CLI's `--once`). Default `false`: the grant persists for the
   * session — the right shape for the probation "one-time OK".
   */
  oneShot?: boolean;
}

/**
 * Grant one tool on one session via the shared authenticated `apiPost`
 * client. Throws `ApiError` (raised by `apiPost`) carrying the gateway's
 * error envelope on any non-2xx — `parseError` handles both the flat
 * `{"error":"<string>"}` shape these session routes use and the nested
 * `{"error":{"code","message"}}` shape.
 */
export async function approveSessionTool(
  sessionId: string,
  tool: string,
  options: ApproveSessionToolOptions = {},
): Promise<SessionToolGrant> {
  const body: Record<string, unknown> = { tool };
  if (options.note && options.note.trim()) body.note = options.note.trim();
  if (options.oneShot) body.one_shot = true;

  return apiPost<SessionToolGrant>(
    `/api/v1/sessions/${encodeURIComponent(sessionId)}/approve`,
    body,
  );
}

/**
 * Grant every tool in `tools` on the session, sequentially, failing
 * fast with the first gateway error. Resolves to the grants in input
 * order.
 */
export async function approveSessionTools(
  sessionId: string,
  tools: string[],
  options: ApproveSessionToolOptions = {},
): Promise<SessionToolGrant[]> {
  const grants: SessionToolGrant[] = [];
  for (const tool of tools) {
    grants.push(await approveSessionTool(sessionId, tool, options));
  }
  return grants;
}
