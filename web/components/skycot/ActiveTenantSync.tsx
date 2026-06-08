"use client";

import { useEffect } from "react";
import { useSkycotSession } from "@/lib/skycot/use-session";
import { setActiveTenantId } from "@/lib/api/tenant-query-key";

/**
 * Invisible bridge that mirrors the session-derived tenant id into the
 * synchronous holder read by the tenant-scoped query-key hash fn
 * (`lib/api/tenant-query-key.ts`).
 *
 * The `queryKeyHashFn` must be synchronous, but the authoritative tenant
 * id lives in the NextAuth session (the `tnt` JWT claim) which is only
 * reachable through React context. This effect keeps the holder current
 * so every cache key is scoped to the live tenant. On sign-out the
 * session resolves to a null tenant and the holder resets to the
 * anonymous namespace — and `TenantCacheGuard` additionally clears the
 * cache on identity change, so the brief un-synced window is also
 * covered by a hard clear.
 *
 * Must render inside both `SessionProvider` (for the session) and
 * `QueryProvider` — wired just inside `QueryProvider` in this app.
 */
export function ActiveTenantSync(): null {
  const { tenantId } = useSkycotSession();

  useEffect(() => {
    setActiveTenantId(tenantId);
  }, [tenantId]);

  return null;
}
