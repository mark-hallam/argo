"use client";

import { ApprovalRateRing } from "@/components/skycot/approvals/ApprovalRateRing";
import { cn } from "@/lib/utils";

/**
 * Low-level KPI strip rendering chrome. Both `ApprovalsKpiStrip`
 * (canonical 5-cell label set) and any future per-vertical strip
 * compose this primitive with their own cell list. The cell shape
 * intentionally allows any label/sub/tone; surface-level conventions
 * (which 5 labels are canonical) live in `ApprovalsKpiStrip`.
 */

export type KpiTone = "neutral" | "warn" | "bad" | "good";

export interface KpiCell {
  label: string;
  value: string;
  /** Optional caption rendered below the value. */
  sub?: string;
  tone?: KpiTone;
  /** Stable testid suffix so callers/tests can target specific cells. */
  testId?: string;
  /** Optional derived ring data for the approval-rate summary row. */
  rate?: {
    headline: string;
    slices: ApprovalRateSlices;
  };
  /** Explanation surfaced when the value is the dash (e.g. why undefined). */
  ariaDescription?: string;
}

export interface ApprovalRateSlices {
  approved: number;
  refused: number;
  expired: number;
}

export interface KpiStripProps {
  cells: KpiCell[];
  rate?: {
    headline: string;
    slices: ApprovalRateSlices;
    animate?: boolean;
  };
  className?: string;
}

const TONE_CLS: Record<KpiTone, string> = {
  neutral: "text-[var(--foreground)]",
  warn: "text-[var(--warning)]",
  bad: "text-[var(--destructive)]",
  good: "text-[var(--success)]",
};

export function KpiStrip({ cells, rate, className }: KpiStripProps) {
  const approvalRateCell = cells.find((c) => c.testId === "approval-rate");
  const displayRate = approvalRateCell ? approvalRateCell.rate : rate;

  return (
    <section
      aria-label="Approval workload metrics"
      data-testid="approvals-kpi-strip"
      className={cn(
        "grid gap-3 sm:grid-cols-2 lg:grid-cols-5",
        className,
      )}
    >
      {cells.map((c) => (
        <article
          key={c.label}
          data-testid={c.testId ? `kpi-${c.testId}` : "kpi-cell"}
          data-tone={c.tone ?? "neutral"}
          aria-description={c.ariaDescription}
          title={c.ariaDescription}
          className={cn(
            "rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)]",
            "px-4 py-3 flex flex-col gap-1",
          )}
        >
          <div className="text-[10.5px] font-semibold uppercase tracking-[0.1em] text-[var(--muted-foreground)]">
            {c.label}
          </div>
          <div
            className={cn(
              "text-[26px] font-light leading-none tabular-nums",
              TONE_CLS[c.tone ?? "neutral"],
            )}
          >
            {c.value}
          </div>
          {c.sub ? (
            <div className="text-[11.5px] text-[var(--muted-foreground)] leading-tight">
              {c.sub}
            </div>
          ) : null}
        </article>
      ))}

      {displayRate ? (
        <article
          data-testid="kpi-rate-ring"
          className={cn(
            "rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)]",
            "px-4 py-3 flex items-center gap-3 sm:col-span-2 lg:col-span-5",
          )}
        >
          <ApprovalRateRing
            approved={displayRate.slices.approved}
            refused={displayRate.slices.refused}
            expired={displayRate.slices.expired}
            animate={approvalRateCell ? false : rate?.animate}
            size={56}
          />
          <div className="flex flex-col gap-0.5 min-w-0">
            <div className="text-[10.5px] font-semibold uppercase tracking-[0.1em] text-[var(--muted-foreground)]">
              7-day approval rate
            </div>
            <div className="text-[22px] font-light leading-none tabular-nums text-[var(--foreground)]">
              {displayRate.headline}
            </div>
            <div className="text-[11.5px] text-[var(--muted-foreground)] leading-tight">
              <span className="text-[var(--success)] font-semibold">
                {Math.round(displayRate.slices.approved * 100)}%
              </span>{" "}
              approved ·{" "}
              <span className="text-[var(--destructive)] font-semibold">
                {Math.round(displayRate.slices.refused * 100)}%
              </span>{" "}
              refused ·{" "}
              <span className="text-[var(--muted-foreground)] font-semibold">
                {Math.round(displayRate.slices.expired * 100)}%
              </span>{" "}
              expired
            </div>
          </div>
        </article>
      ) : null}
    </section>
  );
}
