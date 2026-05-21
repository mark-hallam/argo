/**
 * Workflow save+publish API client
 * (Stream `web-workflow-save-button-real-wire`).
 *
 * Wraps the gateway create-draft endpoint:
 *
 *   POST /api/v1/verticals/{vid}/workflows/{name}
 *     • body: raw `workflow.toml` (text/plain — NOT JSON)
 *     • returns: 201 { vertical_id, name, version_id }
 *
 * The POST body is raw TOML, not JSON — the shared `apiPost` helper
 * would JSON-encode the string, so this module bypasses it and runs
 * its own fetch (same auth + CSRF semantics as `lib/api/client.ts`).
 *
 * After a successful POST the head version is re-fetched to surface
 * the canonical numeric `version` to the toast (the POST response
 * only carries the opaque `version_id` UUID). No mock fallback in
 * strict-production mode — 404/5xx propagates as an error so the UI
 * can show a retry toast.
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

interface CreateResponse {
  vertical_id: string;
  name: string;
  version_id: string;
}

function gatewayBase(): string {
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
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

async function postWorkflowToml(
  path: string,
  body: string,
  editReason?: string,
): Promise<CreateResponse> {
  const headers = new Headers({
    "Content-Type": "application/toml",
    Accept: "application/json",
  });
  const [token, csrf] = await Promise.all([
    readSessionToken(),
    resolveCsrfToken(),
  ]);
  if (token) headers.set("Authorization", `Bearer ${token}`);
  if (csrf) headers.set("X-CSRF-Token", csrf);
  if (editReason) headers.set("X-Argo-Edit-Reason", editReason);
  const url = `${gatewayBase()}${path}`;
  const res = await fetch(url, { method: "POST", headers, body });
  if (!res.ok) throw await parseError(res);
  return (await res.json()) as CreateResponse;
}

export async function saveDraft(args: SaveDraftArgs): Promise<SaveDraftResult> {
  const path = `/api/v1/verticals/${encodeURIComponent(args.vertical_id)}/workflows/${encodeURIComponent(args.name)}`;
  const created = await postWorkflowToml(
    path,
    args.definition_toml,
    args.edit_reason,
  );
  const head = await apiGet<GatewayWorkflowItem>(path);
  const version = Math.max(1, Number(head.version) || 1);
  return { version_id: created.version_id, version };
}
