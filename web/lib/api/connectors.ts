/**
 * Client helpers for the per-worker connector state.
 *
 * The OAuth handshake itself runs at the gateway via `window.open()`
 * (see `use-connect-flow.ts`); these helpers cover the surrounding
 * read/disconnect REST endpoints exposed by the gateway. Endpoints are
 * scoped per worker so the hire flow can attach pre-hire-commit and
 * the worker profile page can show the same state post-hire.
 *
 * v1 endpoints assumed to exist behind `argo-connector`:
 *   GET    /api/v1/workers/{workerId}/connectors/{connectorId}
 *   DELETE /api/v1/workers/{workerId}/connectors/{connectorId}
 * Until the gateway lands the routes both helpers fall back gracefully
 * (status defaults to "not_connected"; disconnect is a no-op).
 */
import { apiDelete, apiGet, ApiError } from "./client";

export type ConnectorStatus =
  | "not_connected"
  | "connecting"
  | "connected"
  | "reauth_needed"
  | "connect_failed";

export interface ConnectorStatusResponse {
  workerId: string;
  connectorId: string;
  status: ConnectorStatus;
  connectedAt?: string;
  reason?: string;
}

/**
 * Fetch the per-worker status of a single connector. Returns
 * `not_connected` on 404 so the v1 UI can render the empty state
 * without surfacing the missing-endpoint error to users.
 */
export async function getConnectorStatus(
  workerId: string,
  connectorId: string,
  init?: { signal?: AbortSignal },
): Promise<ConnectorStatusResponse> {
  try {
    return await apiGet<ConnectorStatusResponse>(
      `/api/v1/workers/${encodeURIComponent(workerId)}/connectors/${encodeURIComponent(connectorId)}`,
      { signal: init?.signal },
    );
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      return { workerId, connectorId, status: "not_connected" };
    }
    throw err;
  }
}

export interface DisconnectResponse {
  workerId: string;
  connectorId: string;
  ok: boolean;
}

/** Revoke a connector for a worker. 404 → treated as already disconnected. */
export async function disconnectConnector(
  workerId: string,
  connectorId: string,
): Promise<DisconnectResponse> {
  try {
    await apiDelete<void>(
      `/api/v1/workers/${encodeURIComponent(workerId)}/connectors/${encodeURIComponent(connectorId)}`,
    );
    return { workerId, connectorId, ok: true };
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      return { workerId, connectorId, ok: true };
    }
    throw err;
  }
}

/**
 * Resolve the base URL for the gateway's OAuth / CU connect endpoints.
 * Mirrors the convention in `lib/api/client.ts` (env override → 8787
 * fallback) but kept local so consumers don't reach into the client
 * internals for a URL builder.
 */
export function gatewayBaseUrl(): string {
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? "http://127.0.0.1:8787";
}

/**
 * Build the gateway URL the popup navigates to. The `return` query
 * param is the URL the gateway redirects back to once OAuth completes;
 * that page posts the result back via `window.opener.postMessage`.
 */
export function buildConnectUrl(args: {
  authType: "oauth" | "cu";
  connectorId: string;
  workerId: string;
  returnOrigin: string;
}): string {
  const base = gatewayBaseUrl();
  const pathPrefix = args.authType === "cu" ? "/connect/cu" : "/connect";
  const ret = `${args.returnOrigin}/hire/oauth-return`;
  const params = new URLSearchParams({
    worker: args.workerId,
    return: ret,
  });
  return `${base}${pathPrefix}/${encodeURIComponent(args.connectorId)}?${params.toString()}`;
}
