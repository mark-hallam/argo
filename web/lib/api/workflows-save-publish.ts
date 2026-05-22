/**
 * Workflow save+publish API client
 * (Streams `web-workflow-save-button-real-wire` + `web-workflow-publish-button-real-wire`).
 *
 * Wraps two gateway endpoints:
 *
 *   POST /api/v1/verticals/{vid}/workflows/{name}
 *     • body: raw `workflow.toml` (text/plain — NOT JSON)
 *     • returns: 201 { vertical_id, name, version_id }
 *
 *   PUT  /api/v1/verticals/{vid}/workflows/{name}/head
 *     • body: { "version_id": "<uuid>" }
 *     • returns: 204 No Content
 *     • side effect: gateway emits an `AuditKind::WorkflowHeadChanged`
 *       audit row (see `routes_workflows_handlers.rs::set_head`).
 *
 * The POST body is raw TOML, not JSON — the shared `apiPost` helper
 * would JSON-encode the string, so this module bypasses it and runs
 * its own fetch (same auth + CSRF semantics as `lib/api/client.ts`).
 * The PUT body is JSON; the workspace has no `apiPut` helper, so the
 * same hand-rolled fetch handles it (keeps both calls coherent and
 * scoped to this module).
 *
 * After a successful POST the head version is re-fetched to surface
 * the canonical numeric `version` to the toast (the POST response
 * only carries the opaque `version_id` UUID). After a successful PUT
 * the head is re-read for the same reason — the canonical numeric
 * `head_version` shown in the toast is the gateway's record of truth,
 * not the version we asked for (covers the edge case where another
 * actor changes head between our call and the read).
 *
 * No mock fallback in strict-production mode — 404/5xx propagates as
 * an error so the UI can show a retry toast.
 */
import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import type { GatewayWorkflowItem } from "@/lib/api/workflows";
import { resolveCsrfToken } from "@/lib/auth/csrf-token";

const DEFAULT_GATEWAY = "http://127.0.0.1:7400";

export interface SaveDraftArgs {
  vertical_id: string;
  name: string;
  definition_toml: string;
  /** Optional edit reason — surfaces as `X-Argo-Edit-Reason` header. */
  edit_reason?: string;
}

export interface SaveDraftResult {
  version_id: string;
  version: number;
}

export interface PublishHeadArgs {
  vertical_id: string;
  name: string;
  /** Version id returned by a prior `saveDraft` call. */
  version_id: string;
}

export interface PublishHeadResult {
  /** Canonical numeric head version after publish, read back from the gateway. */
  head_version: number;
}

interface CreateResponse {
  vertical_id: string;
  name: string;
  version_id: string;
}

function gatewayBase(): string {
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
}

function workflowPath(verticalId: string, name: string): string {
  return `/api/v1/verticals/${encodeURIComponent(verticalId)}/workflows/${encodeURIComponent(name)}`;
}

async function readSessionToken(): Promise<string | undefined> {
  if (typeof window === "undefined") {
    const { auth } = await import("@/auth");
    const session = await auth();
    return session?.gatewayToken;
  }
  try {
    const res = await fetch("/api/auth/session", { credentials: "same-origin" });
    if (!res.ok) return undefined;
    const session = (await res.json()) as { gatewayToken?: string };
    return session?.gatewayToken;
  } catch {
    return undefined;
  }
}

async function parseError(res: Response): Promise<ApiError> {
  const text = await res.text().catch(() => "");
  if (res.status >= 400 && res.status < 500 && text) {
    try {
      const parsed = JSON.parse(text) as {
        error?: { code?: string; message?: string };
      };
      if (parsed?.error?.code || parsed?.error?.message) {
        return new ApiError(
          res.status,
          parsed.error.message ?? res.statusText,
          parsed.error.code,
          parsed,
        );
      }
    } catch {
      /* fall through to raw-body error */
    }
  }
  return new ApiError(
    res.status,
    res.statusText || `HTTP ${res.status}`,
    undefined,
    text || undefined,
  );
}

async function buildAuthHeaders(
  contentType: string,
  editReason?: string,
): Promise<Headers> {
  const headers = new Headers({
    "Content-Type": contentType,
    Accept: "application/json",
  });
  const [token, csrf] = await Promise.all([
    readSessionToken(),
    resolveCsrfToken(),
  ]);
  if (token) headers.set("Authorization", `Bearer ${token}`);
  if (csrf) headers.set("X-CSRF-Token", csrf);
  if (editReason) headers.set("X-Argo-Edit-Reason", editReason);
  return headers;
}

async function postWorkflowToml(
  path: string,
  body: string,
  editReason?: string,
): Promise<CreateResponse> {
  const headers = await buildAuthHeaders("application/toml", editReason);
  const url = `${gatewayBase()}${path}`;
  const res = await fetch(url, { method: "POST", headers, body });
  if (!res.ok) throw await parseError(res);
  return (await res.json()) as CreateResponse;
}

async function putWorkflowJson(path: string, body: unknown): Promise<void> {
  const headers = await buildAuthHeaders("application/json");
  const url = `${gatewayBase()}${path}`;
  const res = await fetch(url, {
    method: "PUT",
    headers,
    body: JSON.stringify(body),
  });
  if (!res.ok) throw await parseError(res);
  // 204 No Content — nothing to parse.
}

export async function saveDraft(args: SaveDraftArgs): Promise<SaveDraftResult> {
  const path = workflowPath(args.vertical_id, args.name);
  const created = await postWorkflowToml(
    path,
    args.definition_toml,
    args.edit_reason,
  );
  const head = await apiGet<GatewayWorkflowItem>(path);
  const version = Math.max(1, Number(head.version) || 1);
  return { version_id: created.version_id, version };
}

export async function publishHead(
  args: PublishHeadArgs,
): Promise<PublishHeadResult> {
  const base = workflowPath(args.vertical_id, args.name);
  await putWorkflowJson(`${base}/head`, { version_id: args.version_id });
  const head = await apiGet<GatewayWorkflowItem>(base);
  const head_version = Math.max(1, Number(head.version) || 1);
  return { head_version };
}
