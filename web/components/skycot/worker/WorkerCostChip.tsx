"use client";

/**
 * Per-teammate cost chip for the worker Overview tab (UF A7, DEC-038).
 *
 * Sits beside the goal-attainment KPIs so the operator reads cost and
 * value together (the cost-vs-value pairing). Reads the tenant-scoped
 * `/api/v1/tenant/costs/by-worker` feed and shows this teammate's row.
 *
 * NO THEATER: renders nothing while loading / on error, and renders a
 * plain `$0.00` when the teammate has no recorded spend — every figure
 * traces to a `spend_ledger` row, none are fabricated.
 */

import { useQuery } from "@tanstack/react-query";
import { DollarSign } from "lucide-react";
import {
  fetchTenantCostByWorker,
  formatUsdMicros,
  type TenantCostByWorker,
} from "@/lib/api/cost";

export function WorkerCostChip({ workerId }: { workerId: string }) {
  const { data, isLoading, isError } = useQuery<TenantCostByWorker>({
    queryKey: ["billing", "cost", "by-worker"],
    queryFn: fetchTenantCostByWorker,
  });

  if (isLoading || isError || !data) return null;

  const micros =
    data.by_worker.find((row) => row.worker_id === workerId)
      ?.spent_usd_micros ?? 0;

  return (
    <div
      data-testid="worker-cost-chip"
      className="inline-flex items-center gap-1.5 rounded-full border border-[var(--border)] bg-[var(--muted)]/40 px-3 py-1 text-sm"
      title="Total provider spend attributed to this teammate"
    >
      <DollarSign aria-hidden className="size-3.5 text-[var(--muted-foreground)]" />
      <span className="text-[var(--muted-foreground)]">Spend</span>
      <span className="font-semibold tabular-nums">
        {formatUsdMicros(micros)}
      </span>
    </div>
  );
}
