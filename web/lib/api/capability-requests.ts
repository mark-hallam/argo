/**
 * Connect-on-demand: pending capability-requests (tg1) + the durable
 * per-tool grant (tg3).
 *
 * When a teammate's run hits a tool/connector its capability fence
 * denies, the runtime records a pending `capability_request` row (one
 * per `(tenant, worker, tool)`). The worker-overview surface polls
 * these via tg1 and renders the locked `G2 · Elevated` grant card for
 * the newest pending ask — turning a silent deny into a one-click
 * "Connect" moment.
 *
 * Two backends sit behind the card's Connect, picked by what the
 * teammate was denied:
 *
 *  - A **runtime tool** (e.g. the `file_read` / `file_write` family) is
 *    just fenced — opening the fence is a durable manifest edit, so it
 *    routes to tg3 `POST .../capability-grants`. The gateway's
 *    `resolve_tool` maps `file` / `filesystem` onto the concrete
 *    `file_read` / `file_write` runtime tool, so the card's read/write
 *    toggle maps straight onto `access`.
 *  - A **connector** (anything in the connector registry — Gmail,
 *    Slack, …) needs a real OAuth/credential handshake, not just a fence
 *    edit, so it keeps the existing tg2 connector-authorize fetcher
 *    (`authorizeConnectorScopes`).
 *
 * The host (`WorkerOverviewContent`) owns the busy/error surface and the
 * mock-gating (`hasRealSession`); these helpers are pure data + a couple
 * of framework-free mappers.
 */

import { apiGet, apiPost, ApiError } from "@/lib/api/client";
import { findConnectorById } from "@/lib/connectors/registry";
import type {
  CapabilityGrantRequest,
  GrantAccess,
  GrantScope,
} from "@/lib/types/capability-grant";

/** One pending row from `GET /api/v1/workers/<id>/capability-requests`. */
export interface CapabilityRequestRow {
  /** Stable request id (the tg1 UUID), forwarded to tg3 as `request_id`. */
  id: string;
  /** The tool or connector name the fence refused. */
  tool_or_connector: string;
  /** Access the worker attempted: `read` or `write`. */
  requested_access: GrantAccess;
  /** The run that triggered the deny, when one is known. */
  run_id: string | null;
  /** Lifecycle status: `pending` | `granted` | `dismissed`. */
  status: string;
  /** RFC3339 timestamp when the request was first recorded. */
  created_utc: string;
  /** RFC3339 timestamp of the most recent deny for this `(worker, tool)`. */
  last_seen_utc: string;
}

/** Result of {@link fetchPendingCapabilityRequests}. */
export interface PendingCapabilityRequests {
  /** Worker display name from the tg1 envelope (`""` when absent). */
  workerName: string;
  /** Pending rows, most-recently-seen first (tg1's own ordering). */
  requests: CapabilityRequestRow[];
}

interface CapabilityRequestsEnvelope {
  worker_id: string;
  worker_name: string;
  capability_requests: CapabilityRequestRow[];
}

/**
 * Fetch the worker's pending capability-requests (tg1).
 *
 * A 404 (gateway without the route, or a worker with none) resolves to
 * an empty list so the overview renders the honest empty state rather
 * than surfacing an error. Any other failure propagates — the caller
 * decides whether a transient poll failure should clear the card.
 */
export async function fetchPendingCapabilityRequests(
  workerId: string,
  init?: { signal?: AbortSignal },
): Promise<PendingCapabilityRequests> {
  try {
    const env = await apiGet<CapabilityRequestsEnvelope>(
      `/api/v1/workers/${encodeURIComponent(workerId)}/capability-requests`,
      { signal: init?.signal },
    );
    const rows = Array.isArray(env?.capability_requests)
      ? env.capability_requests.filter((r) => r.status === "pending")
      : [];
    return {
      workerName: typeof env?.worker_name === "string" ? env.worker_name : "",
      requests: rows,
    };
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      return { workerName: "", requests: [] };
    }
    throw err;
  }
}

/** Result of {@link grantCapability} — mirrors tg3's grant envelope. */
export interface GrantCapabilityResult {
  worker_id: string;
  tool: string;
  access: GrantAccess;
  outcome: string;
}

/**
 * Durable per-tool grant (tg3) — `POST .../capability-grants`. Rewrites
 * the teammate's tenant-scoped Genesis manifest so a previously-fenced
 * tool passes on the NEXT run (revocable any time).
 *
 * Does NOT swallow errors: a failed grant must surface honestly so the
 * card can show the real failure (no silent "connected" theater).
 */
export async function grantCapability(args: {
  workerId: string;
  toolOrConnector: string;
  access: GrantAccess;
  requestId?: string;
}): Promise<GrantCapabilityResult> {
  const body: Record<string, unknown> = {
    tool_or_connector: args.toolOrConnector,
    access: args.access,
  };
  if (args.requestId) body.request_id = args.requestId;
  return apiPost<GrantCapabilityResult>(
    `/api/v1/workers/${encodeURIComponent(args.workerId)}/capability-grants`,
    body,
  );
}

/**
 * `true` when the denied resource is a known connector (needs a real
 * OAuth/credential handshake via tg2), `false` for a runtime tool that
 * a durable fence-edit (tg3) covers.
 */
export function isConnectorRequest(toolOrConnector: string): boolean {
  return findConnectorById(toolOrConnector) !== undefined;
}

/** Friendly labels for the file-family runtime tools tg3 resolves. */
const FILE_TOOL_LABELS: Record<string, string> = {
  file: "Files",
  files: "Files",
  filesystem: "Files",
  file_read: "Files",
  file_write: "Files",
};

function prettyToolLabel(name: string): string {
  const lower = name.toLowerCase();
  if (FILE_TOOL_LABELS[lower]) return FILE_TOOL_LABELS[lower];
  return name
    .replace(/[_-]+/g, " ")
    .replace(/\b\w/g, (c) => c.toUpperCase());
}

/**
 * Map one pending tg1 row onto the locked card's request shape. The
 * card renders a single scope (the newest pending ask); the read/write
 * toggle is seeded from the attempted `requested_access` and stays
 * fully grantable so the operator can widen or narrow it before Connect.
 */
export function capabilityRequestToGrant(
  row: CapabilityRequestRow,
  workerName: string,
): CapabilityGrantRequest {
  const connector = findConnectorById(row.tool_or_connector);
  const displayName = connector?.name ?? prettyToolLabel(row.tool_or_connector);
  const first = workerName.split(" ")[0] || workerName || "your teammate";
  const access: GrantAccess = row.requested_access === "write" ? "write" : "read";
  const scope: GrantScope = {
    id: row.tool_or_connector,
    label: displayName,
    description:
      connector?.description ??
      `${first} needs ${access} access to ${displayName} to finish this task.`,
    access,
    grantable: ["read", "write"],
  };
  return {
    connector: { id: row.tool_or_connector, name: displayName },
    teammateName: workerName || "Your teammate",
    askLine: connector
      ? `Connect ${displayName} so ${first} can keep going`
      : `Let ${first} ${access === "write" ? "work with" : "read"} ${displayName}`,
    scopes: [scope],
  };
}
