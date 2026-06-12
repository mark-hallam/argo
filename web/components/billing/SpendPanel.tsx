"use client";

/**
 * `/billing` dollar-spend panel (UF A7, DEC-038).
 *
 * Shows the tenant's all-time $ spend and the same spend split per AI
 * teammate — the cost-vs-value pairing the operator asked for ("$ per
 * teammate"). Data is tenant-scoped at the gateway
 * (`/api/v1/tenant/costs/{total,by-worker}`), so it is this tenant's
 * spend only.
 *
 * Teammate names come from the server-resolved `worker_name` the
 * by-worker endpoint joins from the worker store; the live roster
 * (`useTeam`) is only a fallback for older gateways that don't send it.
 * A row with a `worker_id` that resolves to neither falls back to
 * "Unknown teammate"; a `null` `worker_id` (system / non-teammate spend)
 * shows as "System". Preferring the server name fixes the "Unknown
 * teammate" label for Genesis hires (shakedown F11), where the roster
 * fetch can fall back to a demo sample or omit archived workers. NO
 * THEATER: every dollar shown traces to a `spend_ledger` row.
 */

import { useQuery } from "@tanstack/react-query";
import { Card } from "@/components/ui/Card";
import { useTeam } from "@/lib/hooks/use-team";
import {
  fetchTenantCostTotal,
  fetchTenantCostByWorker,
  formatUsdMicros,
  type TenantCostTotal,
  type TenantCostByWorker,
} from "@/lib/api/cost";

export function SpendPanel() {
  const totalQuery = useQuery<TenantCostTotal>({
    queryKey: ["billing", "cost", "total"],
    queryFn: fetchTenantCostTotal,
  });
  const byWorkerQuery = useQuery<TenantCostByWorker>({
    queryKey: ["billing", "cost", "by-worker"],
    queryFn: fetchTenantCostByWorker,
  });
  const team = useTeam();

  const nameById = new Map<string, string>(
    (team.data?.workers ?? []).map((w) => [w.id, w.name]),
  );

  const isLoading = totalQuery.isLoading || byWorkerQuery.isLoading;
  const error = totalQuery.error ?? byWorkerQuery.error;
  const rows = byWorkerQuery.data?.by_worker ?? [];

  return (
    <Card className="p-6">
      <header className="flex items-baseline justify-between gap-4">
        <h2 className="text-lg font-semibold tracking-tight">Spend</h2>
        {totalQuery.data && (
          <p className="text-2xl font-semibold tabular-nums">
            {formatUsdMicros(totalQuery.data.spent_usd_micros)}
          </p>
        )}
      </header>
      <p className="mt-1 text-sm text-[var(--muted-foreground)]">
        Real provider spend, attributed per teammate.
      </p>

      {isLoading && <p className="mt-4 text-sm">Loading spend…</p>}
      {error && (
        <p className="mt-4 text-sm text-[var(--destructive)]">
          Failed to load spend: {(error as Error).message}
        </p>
      )}

      {!isLoading && !error && rows.length === 0 && (
        <p className="mt-4 text-sm text-[var(--muted-foreground)]">
          No spend recorded yet. Charges appear here as your teammates work.
        </p>
      )}

      {rows.length > 0 && (
        <ul className="mt-4 divide-y divide-[var(--border)]">
          {rows.map((row) => (
            <li
              key={row.worker_id ?? "system"}
              data-testid="spend-by-worker-row"
              className="flex items-center justify-between py-2 text-sm"
            >
              <span className="truncate">
                {row.worker_id
                  ? (row.worker_name ??
                    nameById.get(row.worker_id) ??
                    "Unknown teammate")
                  : "System"}
              </span>
              <span className="tabular-nums font-medium">
                {formatUsdMicros(row.spent_usd_micros)}
              </span>
            </li>
          ))}
        </ul>
      )}
    </Card>
  );
}
