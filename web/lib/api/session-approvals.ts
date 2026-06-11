/**
 * Session tool-approval API client (day-1 unblock FIX 1).
 *
 * A fresh teammate runs inside a 24-hour probation window: every tool
 * needs a one-time human OK before it can fire. The gateway exposes
 * the grant flow at `POST /sessions/<id>/approve` (#665); this module
 * routes through the Next.js proxy `web/app/api/sessions/[id]/approve`
 * so the session bearer token is attached server-side, mirroring the
 * run-now / freeze proxy idiom in `lib/api/workers.ts`.
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

import { ApiError } from "@/lib/api/types";

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
 * Grant one tool on one session via the Next proxy. Throws `ApiError`
 * carrying the gateway's error envelope message on any non-2xx.
 */
export async function approveSessionTool(
  sessionId: string,
  tool: string,
  options: ApproveSessionToolOptions = {},
): Promise<SessionToolGrant> {
  const body: Record<string, unknown> = { tool };
  if (options.note && options.note.trim()) body.note = options.note.trim();
  if (options.oneShot) body.one_shot = true;

  const res = await fetch(
    `/api/sessions/${encodeURIComponent(sessionId)}/approve`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json" },
      cache: "no-store",
      body: JSON.stringify(body),
    },
  );
  const text = await res.text();
  let parsed: unknown = null;
  if (text) {
    try {
      parsed = JSON.parse(text);
    } catch {
      /* leave parsed null */
    }
  }
  if (!res.ok) {
    throw new ApiError(res.status, errorMessage(parsed) ?? res.statusText, undefined, parsed ?? text);
  }
  return parsed as SessionToolGrant;
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

/**
 * The gateway session routes answer errors as `{"error":"<string>"}`
 * while the Next proxy's own failure envelope is
 * `{"error":{"code","message"}}` — accept both.
 */
function errorMessage(parsed: unknown): string | undefined {
  if (!parsed || typeof parsed !== "object" || !("error" in parsed)) {
    return undefined;
  }
  const err = (parsed as { error: unknown }).error;
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) {
    const msg = (err as { message?: unknown }).message;
    if (typeof msg === "string") return msg;
  }
  return undefined;
}
