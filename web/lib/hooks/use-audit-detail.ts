"use client";

import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { fetchAuditDetail, AuditDetailNotFoundError } from "@/lib/api/audit-detail";
import type { AuditDetail } from "@/lib/types/audit-detail";

/**
 * Drill-in detail query. Cache key is namespaced by actionId so
 * sibling routes (the audit feed, replay) can re-use the entry.
 *
 * The 404→not-found path surfaces a typed `AuditDetailNotFoundError`
 * to the caller; the page treats it as the "render <AuditActionNotFound>"
 * branch instead of the generic error state.
 */
export const auditDetailQueryKey = (actionId: string) =>
  ["audit-detail", actionId] as const;

export function useAuditDetail(
  actionId: string,
  options?: { enabled?: boolean },
): UseQueryResult<AuditDetail, Error> {
  return useQuery<AuditDetail, Error>({
    queryKey: auditDetailQueryKey(actionId),
    queryFn: () => fetchAuditDetail(actionId),
    enabled: (options?.enabled ?? true) && actionId.length > 0,
    retry: (failureCount, err) => {
      if (err instanceof AuditDetailNotFoundError) return false;
      return failureCount < 2;
    },
  });
}

export { AuditDetailNotFoundError };
