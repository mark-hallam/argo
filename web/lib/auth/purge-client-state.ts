import type { QueryClient } from "@tanstack/react-query";

/**
 * Client-side storage namespaces that hold TENANT- or SESSION-scoped
 * state and therefore must be purged on an identity change. Device-only
 * preferences use a different shape (`skycot-color-mode`, hyphen) and are
 * intentionally NOT matched, so a sign-out doesn't reset the user's
 * dark/light choice.
 */
const TENANT_STORAGE_PREFIXES = ["skycot:", "skycot."] as const;

function purgeNamespacedStorage(store: Storage | undefined): void {
  if (!store) return;
  const toRemove: string[] = [];
  for (let i = 0; i < store.length; i += 1) {
    const key = store.key(i);
    if (key && TENANT_STORAGE_PREFIXES.some((p) => key.startsWith(p))) {
      toRemove.push(key);
    }
  }
  for (const key of toRemove) {
    try {
      store.removeItem(key);
    } catch {
      /* quota / private-mode — best-effort */
    }
  }
}

/**
 * Wipe every piece of client-side tenant/session state on an identity
 * change (sign-out, or a different principal signing in on the same
 * browser).
 *
 * **First principles:** the signed-in identity is the cache key for the
 * whole client. A new principal must never be able to observe the
 * previous one's data — not the in-memory React Query cache (workers,
 * approvals, audit rows, worker memory…), and not the namespaced
 * `localStorage`/`sessionStorage` (the elevated-view PII grant, the
 * onboarding draft carrying the workspace/tenant id, the genesis tenant
 * slug, the demo roster, the tenant theme…). Clearing the cookie alone
 * is insufficient: a SPA keeps a long-lived `QueryClient` for the whole
 * tab lifetime, so without this purge the next user sees the previous
 * tenant's cached data during the stale-time window.
 *
 * Safe to call on the server (no-ops without `window`).
 */
export function purgeClientTenantState(queryClient: QueryClient): void {
  try {
    queryClient.clear();
  } catch {
    /* defensive — never let a purge failure block sign-out */
  }
  if (typeof window === "undefined") return;
  purgeNamespacedStorage(window.localStorage);
  purgeNamespacedStorage(window.sessionStorage);
}
