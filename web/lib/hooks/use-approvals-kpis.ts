"use client";

import { useMemo } from "react";
import {
  computeApprovalsKpis,
  type ApprovalsKpis,
} from "@/components/skycot/approvals/ApprovalsKpiStrip";
import type { ApprovalItem } from "@/lib/sample-approvals";

/**
 * Thin useMemo wrapper around `computeApprovalsKpis` so both the
 * root inbox (`/`) and the standalone landing (`/approvals`) derive
 * the canonical 5-KPI shape from the same code path.
 *
 * Callers pass the visible (filtered) item list; the hook does not
 * fetch — that's `useApprovals()`'s job. `now` is overridable for
 * tests; in production it defaults to `Date.now()` per render so
 * the median-age math stays live.
 */
export function useApprovalsKpis(
  items: ApprovalItem[],
  now?: number,
): ApprovalsKpis {
  return useMemo(() => computeApprovalsKpis(items, now), [items, now]);
}
