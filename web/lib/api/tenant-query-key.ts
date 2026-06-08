"use client";

/**
 * Tenant-scoped React Query cache keys (post-launch security hardening).
 *
 * **Root cause this closes:** the SPA holds ONE long-lived `QueryClient`
 * for the whole tab lifetime, and almost every hook keys its queries off
 * a GLOBAL constant (`SKILLS_KEY`, `TEAM_KEY`, `APPROVALS_KEY`,
 * `workerMemoryQueryKey(id)`, …). Cache identity in TanStack Query is the
 * *hash of the query key*. If two principals (or two workspaces of the
 * same principal via an in-session switcher) share a tab, an identical
 * global key hashes to the identical cache entry — so the second tenant
 * could observe the first tenant's cached rows during the stale-time
 * window. The sign-out / identity-change purge
 * (`purgeClientTenantState`, `TenantCacheGuard`) already clears the cache
 * on an identity *change*; this module is the belt-and-suspenders that
 * makes the alias *impossible by construction*, even with no clear, and
 * is forward-looking for an in-session workspace switcher (which changes
 * tenant WITHOUT an identity change, so the purge would not fire).
 *
 * **First-principles design — completeness by construction.** Rather than
 * thread a `tenantId` argument through ~83 `queryKey:` call sites across
 * ~36 files (where a single missed query/invalidation pair silently
 * breaks cache updates), the tenant is folded in at the QueryClient level
 * via a default {@link tenantQueryKeyHashFn}. The hash function runs for
 * EVERY query — current and future — so coverage cannot regress and no
 * call site can opt out by accident. The explicit {@link tenantKey}
 * helper is provided for hooks that prefer to scope their key directly
 * (and is what the unit test exercises); both routes converge on the same
 * `["t", tenantId, …]` shape, so a `tenantKey(...)` query and a bare
 * global-constant query are scoped to the same tenant namespace.
 *
 * **Why a module-level holder.** A `queryKeyHashFn` must be synchronous,
 * but the authoritative tenant id lives in the NextAuth session
 * (`tnt` JWT claim) read through React context, and the workspace
 * override (`setTenantOverride`) lives in `lib/api/tenant.ts`. The active
 * tenant is therefore mirrored into a synchronously-readable holder that
 * {@link useSyncActiveTenant} keeps in step with `useSkycotSession()` /
 * the workspace override. The holder is the single source the hash fn and
 * `tenantKey` read.
 */

import { getTenantOverride } from "./tenant";

/** Sentinel namespace for "no resolved tenant yet" (logged out, or
 *  pre-hydration). Distinct from any real tenant id so an anonymous
 *  cache entry can never alias an authenticated one. */
export const ANON_TENANT = "anon" as const;

/** Reserved first element marking an already-tenant-scoped key. */
const TENANT_KEY_PREFIX = "t" as const;

/**
 * Synchronously-readable "current tenant" the hash fn folds into every
 * cache key. Mirrors the session-derived tenant id (kept in sync by
 * {@link useSyncActiveTenant}); the workspace-switcher override in
 * `lib/api/tenant.ts` takes precedence so a pinned workspace scopes the
 * cache even before the session-sync effect runs.
 */
let activeTenantId: string | null = null;

/** Resolve the tenant the cache is currently scoped to. Override wins so
 *  an in-session workspace switch is reflected immediately. */
export function getActiveTenantId(): string {
  const override = getTenantOverride();
  if (override && override.length > 0) return override;
  if (activeTenantId && activeTenantId.length > 0) return activeTenantId;
  return ANON_TENANT;
}

/**
 * Mirror the session-derived tenant id into the holder. Called by
 * {@link useSyncActiveTenant}; exposed for tests + non-React callers.
 * Passing `null`/empty resets to the anonymous namespace.
 */
export function setActiveTenantId(tenantId: string | null | undefined): void {
  activeTenantId = tenantId && tenantId.length > 0 ? tenantId : null;
}

/** True when `key` already carries the `["t", tenantId, …]` prefix, so
 *  the hash fn does not double-scope an explicitly-built key. */
function isTenantScoped(key: ReadonlyArray<unknown>): boolean {
  return key.length >= 2 && key[0] === TENANT_KEY_PREFIX;
}

/**
 * Build a tenant-scoped query key: `["t", <tenant>, ...parts]`.
 *
 * Use anywhere a query/mutation key is constructed by hand. Two
 * different active tenants yield different keys for the same `parts`
 * (no aliasing); the same tenant is stable (cache hits work). When no
 * tenant is resolved the key is scoped to {@link ANON_TENANT}, which is
 * still distinct from every real tenant.
 */
export function tenantKey(...parts: ReadonlyArray<unknown>): unknown[] {
  return [TENANT_KEY_PREFIX, getActiveTenantId(), ...parts];
}

/**
 * Default `queryKeyHashFn` for the app's `QueryClient`. Folds the active
 * tenant into the HASH of every key that is not already tenant-scoped,
 * so all hooks — including those still keyed off bare global constants —
 * get a per-tenant cache namespace without touching their call sites.
 *
 * Mirrors TanStack's built-in stable-stringify (object keys sorted) so
 * hashing semantics are otherwise identical to the default.
 */
export function tenantQueryKeyHashFn(queryKey: ReadonlyArray<unknown>): string {
  const scoped = isTenantScoped(queryKey)
    ? queryKey
    : [TENANT_KEY_PREFIX, getActiveTenantId(), ...queryKey];
  return JSON.stringify(scoped, (_key, value) =>
    isPlainObject(value)
      ? Object.keys(value)
          .sort()
          .reduce<Record<string, unknown>>((acc, k) => {
            acc[k] = (value as Record<string, unknown>)[k];
            return acc;
          }, {})
      : value,
  );
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  if (typeof value !== "object" || value === null) return false;
  const proto = Object.getPrototypeOf(value) as unknown;
  return proto === Object.prototype || proto === null;
}
