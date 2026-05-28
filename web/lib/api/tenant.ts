/**
 * Tenant-id resolver for SPA → gateway requests.
 *
 * The gateway (`crates/argo-cli/src/handlers/gateway/routes_auth.rs`)
 * accepts `X-Argo-Tenant-Id` on every request and validates it
 * against the bearer-token / session-derived tenant:
 *
 *   - Header absent → gateway falls back to the principal-derived
 *     tenant (today's single-workspace-per-session behaviour).
 *   - Header supplied + matches the bound token's tenant → respected.
 *   - Header supplied + mismatches → 403 `tenant-header mismatch`
 *     (see `enforce_header_match`).
 *
 * This module is the SPA-side resolver wired by `client.ts`. Today
 * it returns whatever the session JSON happens to expose as
 * `tenantId` (undefined for the current single-workspace JWT), so
 * the header is omitted and the gateway's principal-derived fallback
 * keeps working — no observable change to today's flows.
 *
 * When the workspace-switcher track lands, the switcher calls
 * `setTenantOverride(workspaceTenantId)` to pin the active workspace.
 * The resolver then returns that id and the client stamps it on every
 * request so the user can be a member of multiple tenants while
 * keeping their user-level identity stable.
 */

let tenantOverride: string | undefined;

/** Pin the active workspace's tenant id. Workspace-switcher entry point. */
export function setTenantOverride(id: string | undefined): void {
  tenantOverride = id && id.length > 0 ? id : undefined;
}

/** Drop the workspace override (e.g. on sign-out). */
export function clearTenantOverride(): void {
  tenantOverride = undefined;
}

/** Inspect the current override. Exposed for tests + workspace-switcher UI. */
export function getTenantOverride(): string | undefined {
  return tenantOverride;
}

/**
 * Resolve the tenant id to stamp on the next request.
 *
 * Precedence:
 *   1. workspace-switcher override (set via `setTenantOverride`)
 *   2. `peek.tenantId` from the session payload (workspace-switcher
 *      track may populate this from the magic-link JWT)
 *   3. `undefined` → omit the header; gateway falls back to the
 *      principal-derived tenant.
 */
export function resolveTenantId(peek: { tenantId?: string }): string | undefined {
  if (tenantOverride) return tenantOverride;
  if (peek.tenantId && peek.tenantId.length > 0) return peek.tenantId;
  return undefined;
}
