"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState, type ReactNode } from "react";

/**
 * Single TanStack Query client for the app. Defaults tuned for the
 * approval-card surface: stale-while-revalidate cadence, retry on
 * transient errors only, idempotent mutations.
 *
 * The real gateway wire-up happens in lib/api/approvals.ts —
 * useApprovals() reads from there. Replace the fetcher in that
 * file when the JSON `/inbox` endpoint lands.
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

  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}
