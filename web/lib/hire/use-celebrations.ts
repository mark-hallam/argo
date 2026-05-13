"use client";

import { useCallback, useEffect, useState } from "react";
import { useApprovals } from "@/lib/hooks/use-approvals";
import {
  clearCelebration as clearCelebrationStore,
  getCelebrations,
  pruneExpired,
  type CelebrationEntry,
} from "./celebration-storage";

export interface UseCelebrationsResult {
  /** Unexpired celebrations in hire order. Empty until first client render. */
  entries: CelebrationEntry[];
  /** Remove the entry for `workerId` and update local state. */
  dismiss: (workerId: string) => void;
}

/**
 * React hook bridging celebration storage with the approvals cache.
 *
 * - Defers localStorage reads until after first mount (SSR-safe; no
 *   hydration mismatch since the server-render gives `entries=[]`).
 * - Watches the approvals query for the first approval that matches a
 *   celebrated worker — when one arrives, the celebration auto-clears
 *   (the approval card itself is the "first opinion").
 *
 * Approval items don't carry an opaque `worker.id`; v1 matches by
 * `worker.name === workerName`. The architectural target swaps for
 * server-supplied worker ids — tracked under
 * `mission-enterprise-hire-post-state`.
 */
export function useCelebrations(): UseCelebrationsResult {
  const [entries, setEntries] = useState<CelebrationEntry[]>([]);
  const approvalsQuery = useApprovals();

  useEffect(() => {
    pruneExpired();
    setEntries(getCelebrations());
  }, []);

  const items = approvalsQuery.data ?? [];

  useEffect(() => {
    if (entries.length === 0 || items.length === 0) return;
    const matchedIds: string[] = [];
    for (const entry of entries) {
      const hasOpinion = items.some(
        (item) => item.worker.name === entry.workerName,
      );
      if (hasOpinion) matchedIds.push(entry.workerId);
    }
    if (matchedIds.length === 0) return;
    for (const id of matchedIds) clearCelebrationStore(id);
    setEntries((prev) => prev.filter((e) => !matchedIds.includes(e.workerId)));
  }, [entries, items]);

  const dismiss = useCallback((workerId: string) => {
    clearCelebrationStore(workerId);
    setEntries((prev) => prev.filter((e) => e.workerId !== workerId));
  }, []);

  return { entries, dismiss };
}
