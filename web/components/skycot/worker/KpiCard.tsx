import { ArrowDownRight, ArrowUpRight, Minus } from "lucide-react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { KpiSparkline } from "@/components/ui/charts";
import { cn } from "@/lib/utils";
import type { KpiTone, WorkerKpi } from "@/lib/api/worker-overview";

/**
 * KPI block — 4 rows of label · value · 7-day sparkline · trend arrow + delta.
 *
 * KPI selection is vertical-aware: the parent passes whatever the
 * gateway returned in `worker.kpiSet[]` (or the mock fallback for
 * dev). This component is presentation-only; it does not pick KPIs.
 *
 * Trend arrow colour mirrors `kpi.tone`:
 *   - success (green) when trend is "up" (more is better)
 *   - warning (amber/red) when trend is "down"
 *   - muted when trend is "flat" or the KPI series is absent
 */

export interface KpiCardProps {
  kpis: WorkerKpi[];
  error?: string | null;
  onRetry?: () => void;
  className?: string;
}

export function KpiCard({ kpis, error, onRetry, className }: KpiCardProps) {
  return (
    <Card data-testid="card-kpis" className={className}>
      <CardHeader>
        <CardTitle>Today&apos;s KPIs</CardTitle>
      </CardHeader>
      <CardBody className="pt-1">
        {error ? (
          <KpiErrorState message={error} onRetry={onRetry} />
        ) : kpis.length === 0 ? (
          <p className="text-sm text-[var(--muted-foreground)]">
            No KPIs published for this worker yet.
          </p>
        ) : (
          <ul role="list" className="divide-y divide-[var(--border)]/60">
            {kpis.map((k) => (
              <KpiRow key={k.id} kpi={k} />
            ))}
          </ul>
        )}
      </CardBody>
    </Card>
  );
}

function KpiRow({ kpi }: { kpi: WorkerKpi }) {
  return (
    <li
      data-testid={`kpi-row-${kpi.id}`}
      data-tone={kpi.tone}
      className="grid grid-cols-[1fr_auto] sm:grid-cols-[1.6fr_auto_auto_auto] items-center gap-x-4 gap-y-1 py-3"
    >
      <p className="text-sm text-[var(--muted-foreground)]">{kpi.label}</p>
      <p
        data-testid={`kpi-value-${kpi.id}`}
        className="justify-self-end sm:justify-self-start text-base font-semibold tabular-nums text-[var(--foreground)]"
      >
        {kpi.value}
      </p>
      <KpiSparkline
        data={kpi.series}
        tone={kpi.tone}
        ariaLabel={`7-day trend for ${kpi.label}`}
        className="col-span-2 sm:col-span-1 justify-self-stretch sm:justify-self-center"
      />
      <KpiTrend trend={kpi.trend} tone={kpi.tone} deltaPct={kpi.deltaPct} />
    </li>
  );
}

function KpiTrend({
  trend,
  tone,
  deltaPct,
}: {
  trend: WorkerKpi["trend"];
  tone: KpiTone;
  deltaPct: number;
}) {
  const Icon = trend === "up" ? ArrowUpRight : trend === "down" ? ArrowDownRight : Minus;
  const toneClass = toneTextClass(tone);
  const sign = deltaPct > 0 ? "+" : "";
  return (
    <span
      data-testid="kpi-trend"
      data-trend={trend}
      className={cn(
        "col-span-2 sm:col-span-1 justify-self-end inline-flex items-center gap-1 text-xs font-medium tabular-nums",
        toneClass,
      )}
    >
      <Icon className="size-3.5" aria-hidden />
      <span aria-label={`${sign}${deltaPct} percent vs previous period`}>
        {sign}
        {deltaPct}%
      </span>
    </span>
  );
}

function toneTextClass(tone: KpiTone): string {
  switch (tone) {
    case "success":
      return "text-[var(--success)]";
    case "warning":
      return "text-[var(--warning)]";
    case "muted":
    default:
      return "text-[var(--muted-foreground)]";
  }
}

function KpiErrorState({
  message,
  onRetry,
}: {
  message: string;
  onRetry?: () => void;
}) {
  return (
    <div className="space-y-2 text-sm text-[var(--muted-foreground)]">
      <p>{message}</p>
      {onRetry ? (
        <button
          type="button"
          onClick={onRetry}
          className="text-[var(--primary)] underline-offset-4 hover:underline"
        >
          Retry
        </button>
      ) : null}
    </div>
  );
}

export function KpiCardSkeleton() {
  return (
    <Card data-testid="card-kpis-skeleton">
      <CardHeader>
        <div className="h-4 w-28 rounded bg-[var(--muted)]" />
      </CardHeader>
      <CardBody className="pt-1 space-y-3">
        {[0, 1, 2, 3].map((i) => (
          <div key={i} className="flex items-center gap-3">
            <div className="flex-1 h-3 rounded bg-[var(--muted)]" />
            <div className="h-3 w-10 rounded bg-[var(--muted)]" />
            <div className="h-6 w-20 rounded bg-[var(--muted)]" />
            <div className="h-3 w-10 rounded bg-[var(--muted)]" />
          </div>
        ))}
      </CardBody>
    </Card>
  );
}
