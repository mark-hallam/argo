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
import { apiDelete, apiGet, apiPost, ApiError } from "./client";
import { resolveGatewayUrl } from "@/lib/config/gateway-url";
import type { GrantSelection } from "@/lib/types/capability-grant";

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
 * Delegates to the canonical `gateway-url` module so this builder
 * shares the same source of truth as the CSP `connect-src` directive.
 */
export function gatewayBaseUrl(): string {
  return resolveGatewayUrl();
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

/**
 * Tenant-scoped OAuth-start URL for the `/settings/connectors` page.
 * Mirrors {@link buildConnectUrl} but without the `worker` query
 * parameter — the gateway scopes the saved credential to the tenant
 * directly. Returns to `/settings/connectors/oauth-return`.
 */
export function buildTenantConnectUrl(args: {
  connectorId: string;
  returnOrigin: string;
}): string {
  const base = gatewayBaseUrl();
  const ret = `${args.returnOrigin}/settings/connectors/oauth-return`;
  const params = new URLSearchParams({ return: ret });
  return `${base}/connect/${encodeURIComponent(args.connectorId)}?${params.toString()}`;
}

/** Single row in the tenant-scoped connector status response. */
export interface TenantConnectorRow {
  connectorId: string;
  status: ConnectorStatus;
  connectedAs?: string;
  connectedAt?: string;
}

export interface TenantConnectorListResponse {
  connectors: TenantConnectorRow[];
}

/**
 * Read the tenant's connection state for every connector at once.
 * Falls back to an empty list when the gateway route is missing so
 * the catalog page still renders during v1 rollout.
 */
export async function listTenantConnectors(
  init?: { signal?: AbortSignal },
): Promise<TenantConnectorListResponse> {
  try {
    return await apiGet<TenantConnectorListResponse>("/api/v1/connectors", {
      signal: init?.signal,
    });
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      return { connectors: [] };
    }
    throw err;
  }
}

/** Revoke a tenant-scoped credential. 404 → already disconnected. */
export async function disconnectTenantConnector(
  connectorId: string,
): Promise<{ connectorId: string; ok: boolean }> {
  try {
    await apiDelete<void>(
      `/api/v1/connectors/${encodeURIComponent(connectorId)}`,
    );
    return { connectorId, ok: true };
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      return { connectorId, ok: true };
    }
    throw err;
  }
}

/**
 * Save a credential for an API-key-shaped connector. The gateway is
 * responsible for the encrypted-at-rest persistence + tenant scoping.
 */
export async function saveTenantConnectorCredential(args: {
  connectorId: string;
  fields: Record<string, string>;
}): Promise<{ ok: true }> {
  await apiPost<void>(
    `/api/v1/connectors/${encodeURIComponent(args.connectorId)}/credentials`,
    args.fields,
  );
  return { ok: true };
}

export interface AuthorizeConnectorScopesResponse {
  workerId: string;
  connectorId: string;
  ok: boolean;
}

/**
 * Connect-on-demand authorize: grant a teammate the chosen per-scope
 * access on a connector it hit mid-run. Backs the locked
 * `CapabilityGrantCard` "Connect" action — POSTs the connector id + the
 * operator's `{id, access}[]` selection to the gateway's
 * connector-authorize path, tenant- and worker-scoped.
 *
 * Unlike the read/disconnect helpers above, this does NOT swallow
 * errors: a failed authorize must surface honestly so the card can show
 * the real failure and the operator knows the connect didn't happen
 * (no silent "connected" theater).
 */
export async function authorizeConnectorScopes(args: {
  workerId: string;
  connectorId: string;
  scopes: GrantSelection[];
}): Promise<AuthorizeConnectorScopesResponse> {
  await apiPost<void>(
    `/api/v1/workers/${encodeURIComponent(args.workerId)}/connectors/${encodeURIComponent(args.connectorId)}/authorize`,
    { scopes: args.scopes },
  );
  return { workerId: args.workerId, connectorId: args.connectorId, ok: true };
}
