"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState, type ReactNode } from "react";
import { tenantQueryKeyHashFn } from "@/lib/api/tenant-query-key";
import { ActiveTenantSync } from "./ActiveTenantSync";

/**
 * Single TanStack Query client for the app. Defaults tuned for the
 * approval-card surface: stale-while-revalidate cadence, retry on
 * transient errors only, idempotent mutations.
 *
 * The real gateway wire-up happens in lib/api/approvals.ts —
 * useApprovals() reads from there. Replace the fetcher in that
 * file when the JSON `/inbox` endpoint lands.
 *
 * **Tenant cache scoping (security).** `queryKeyHashFn` folds the active
 * tenant into the hash of EVERY query key, so no two tenants sharing a
 * tab can ever alias the same cache entry — even for hooks still keyed
 * off bare global constants, and even without a sign-out clear. See
 * `lib/api/tenant-query-key.ts`. `ActiveTenantSync` keeps the tenant the
 * hash fn reads in step with the session.
 */
export function QueryProvider({ children }: { children: ReactNode }) {
  // Lazy-init so we don't share a client across SSR requests.
  const [client] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: 30_000,
            refetchOnWindowFocus: true,
            queryKeyHashFn: tenantQueryKeyHashFn,
            retry: (count, err) => {
              // 4xx are real errors; retry only on network / 5xx.
              const status = (err as { status?: number })?.status;
              if (status && status >= 400 && status < 500) return false;
              return count < 2;
            },
          },
          mutations: { retry: false },
        },
      }),
  );

  return (
    <QueryClientProvider client={client}>
      <ActiveTenantSync />
      {children}
    </QueryClientProvider>
  );
}
