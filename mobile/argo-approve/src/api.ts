import type { ApprovalRow, Decision, InboxSnapshot } from "./types";

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

function authHeader(bearer: string): HeadersInit {
  return bearer ? { Authorization: `Bearer ${bearer}` } : {};
}

function joinUrl(base: string, path: string): string {
  const trimmed = base.replace(/\/+$/, "");
  return `${trimmed}${path}`;
}

export async function fetchPending(
  gatewayUrl: string,
  bearer: string,
): Promise<ApprovalRow[]> {
  const res = await fetch(joinUrl(gatewayUrl, "/inbox/ws"), {
    method: "GET",
    headers: { Accept: "application/json", ...authHeader(bearer) },
  });
  if (!res.ok) {
    throw new ApiError(res.status, `GET /inbox/ws → ${res.status}`);
  }
  const text = await res.text();
  const snapshot = parseSnapshot(text);
  return snapshotToRows(snapshot);
}

function parseSnapshot(raw: string): InboxSnapshot {
  try {
    const value = JSON.parse(raw);
    if (value && value.type === "snapshot" && Array.isArray(value.events)) {
      return value as InboxSnapshot;
    }
  } catch {
    /* fall through to empty snapshot */
  }
  return { type: "snapshot", events: [] };
}

function snapshotToRows(snapshot: InboxSnapshot): ApprovalRow[] {
  return snapshot.events.map((e) => ({
    approval_id: e.approval_id,
    action_id: e.action_id,
    risk_level: e.risk_level,
    prompt_body: "",
    expiry: "",
    enqueued_at: "",
  }));
}

export async function decide(
  gatewayUrl: string,
  bearer: string,
  approvalId: string,
  decision: Decision,
): Promise<void> {
  const path = decision === "approve"
    ? `/inbox/approve/${encodeURIComponent(approvalId)}`
    : `/inbox/reject/${encodeURIComponent(approvalId)}`;
  const res = await fetch(joinUrl(gatewayUrl, path), {
    method: "POST",
    headers: { Accept: "application/json", ...authHeader(bearer) },
  });
  if (!res.ok) {
    const body = await res.text().catch(() => "");
    throw new ApiError(res.status, `POST ${path} → ${res.status} ${body}`);
  }
}

export async function decideViaPushToken(
  gatewayUrl: string,
  token: string,
  decision: Decision,
): Promise<void> {
  const path = `/channels/push/decision/${encodeURIComponent(token)}?d=${decision}`;
  const res = await fetch(joinUrl(gatewayUrl, path), { method: "POST" });
  if (!res.ok) {
    const body = await res.text().catch(() => "");
    throw new ApiError(res.status, `POST ${path} → ${res.status} ${body}`);
  }
}
