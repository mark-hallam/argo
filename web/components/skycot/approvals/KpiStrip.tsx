"use client";

import { ApprovalRateRing } from "@/components/skycot/approvals/ApprovalRateRing";
import type { ApprovalItem } from "@/lib/sample-approvals";
import { cn } from "@/lib/utils";

function formatUsd(amount: number): string {
  if (amount >= 1_000_000) return `$${(amount / 1_000_000).toFixed(1)}M`;
  if (amount >= 10_000) return `$${Math.round(amount / 1_000)}k`;
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    maximumFractionDigits: 0,
  }).format(amount);
}

const WEEK_MS = 7 * 24 * 60 * 60 * 1000;
const SLA_TARGET_MS = 2 * 60 * 60 * 1000;

function parseTimestamp(iso?: string | null): number | null {
  if (!iso) return null;
  const t = new Date(iso).getTime();
  return Number.isFinite(t) ? t : null;
}

function formatDuration(ms: number): string {
  const minutes = Math.max(1, Math.round(ms / 60_000));
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h`;
  return `${Math.round(hours / 24)}d`;
}

function mean(values: number[]): number | null {
  if (values.length === 0) return null;
  return values.reduce((acc, v) => acc + v, 0) / values.length;
}

function formatPercent(value: number): string {
  return `${Math.round(value * 100)}%`;
}

/**
 * Roll-up of the 5 KPI cells from an in-memory ApprovalItem list.
 * Every value is derived from the visible approval rows so brand-new
 * tenants do not inherit stale demo baselines.
 *
 * Lives in the KpiStrip module so the heavy compute stays out of
 * `app/(app)/page.tsx` (LOC budget).
 */
export function computeKpiCells(
  items: ApprovalItem[],
  pendingCount: number,
  now: number = Date.now(),
): KpiCell[] {
  const pendingItems = items.filter((a) => a.decision == null);
  const recentItems = items.filter((a) => {
    const receivedAt = parseTimestamp(a.receivedAt);
    return receivedAt != null && receivedAt >= now - WEEK_MS && receivedAt <= now;
  });
  const decidedItems = recentItems.filter((a) => a.decision != null);
  const approvedCount = decidedItems.filter((a) => a.decision === "approve").length;
  const refusedCount = decidedItems.filter((a) => a.decision === "refuse").length;
  const expiredCount = recentItems.filter((a) => {
    if (a.decision != null) return false;
    const expiresAt = parseTimestamp(a.slaExpiresAt);
    return expiresAt != null && expiresAt <= now;
  }).length;
  const approvalRate =
    decidedItems.length > 0 ? approvedCount / decidedItems.length : null;
  const rateSliceTotal = Math.max(1, approvedCount + refusedCount + expiredCount);
  const responseMs = mean(
    decidedItems.flatMap((a) => {
      const receivedAt = parseTimestamp(a.receivedAt);
      return receivedAt == null ? [] : [Math.max(0, now - receivedAt)];
    }),
  );
  const totalAtRisk = pendingItems.reduce(
    (acc, a) => acc + (a.dollarAtRisk ?? 0),
    0,
  );
  const overSla = pendingItems.filter(
    (a) => a.slaExpiresAt && new Date(a.slaExpiresAt).getTime() <= now,
  ).length;
  const monetaryCount = pendingItems.filter(
    (a) => (a.dollarAtRisk ?? 0) > 0,
  ).length;

  return [
    {
      label: "Pending",
      value: String(pendingCount),
      sub: overSla > 0 ? `${overSla} over SLA · needs eyes` : "across your workforce",
      tone: overSla > 0 ? "bad" : "neutral",
      testId: "pending",
    },
    {
      label: "Resolved / wk",
      value: decidedItems.length > 0 ? String(decidedItems.length) : "—",
      sub:
        decidedItems.length > 0
          ? "from resolved rows"
          : "no decisions in 7 days",
      testId: "resolved",
    },
    {
      label: "Avg response",
      value: responseMs == null ? "—" : formatDuration(responseMs),
      sub: responseMs == null ? "no response history" : "from resolved rows",
      tone:
        responseMs == null
          ? "neutral"
          : responseMs <= SLA_TARGET_MS
            ? "good"
            : "warn",
      testId: "avg-response",
    },
    {
      label: "$ at risk",
      value: totalAtRisk > 0 ? formatUsd(totalAtRisk) : "—",
      sub:
        monetaryCount > 0
          ? `across ${monetaryCount} pending`
          : "no monetary impact",
      tone: totalAtRisk > 0 ? "warn" : "neutral",
      testId: "at-risk",
    },
    {
      label: "Approval rate",
      value: approvalRate == null ? "—" : formatPercent(approvalRate),
      sub: approvalRate == null ? "no decision history" : "from resolved rows",
      tone:
        approvalRate == null ? "neutral" : approvalRate >= 0.5 ? "good" : "warn",
      testId: "approval-rate",
      rate:
        approvalRate == null
          ? undefined
          : {
              headline: formatPercent(approvalRate),
              slices: {
                approved: approvedCount / rateSliceTotal,
                refused: refusedCount / rateSliceTotal,
                expired: expiredCount / rateSliceTotal,
              },
            },
    },
  ];
}

/**
 * 5-cell KPI strip shown above the approvals inbox.
 * Locked design: enterprise-restrained register.
 *
 *  1. Pending decisions   (count)
 *  2. Resolved this week  (count)
 *  3. Avg response        (short duration string)
 *  4. $ at risk           (currency)
 *  5. Approval rate (7-day) — pairs with the donut ring on the right.
 *
 * Each cell carries a small `tone` flag so over-SLA / breached metrics
 * can switch to a destructive accent without changing layout.
 */

export type KpiTone = "neutral" | "warn" | "bad" | "good";

export interface KpiCell {
  label: string;
  value: string;
  /** Optional caption rendered below the value (e.g. "↓ 3m vs last week"). */
  sub?: string;
  tone?: KpiTone;
  /** Stable testid suffix so callers/tests can target specific cells. */
  testId?: string;
  /** Optional derived ring data for the approval-rate summary row. */
  rate?: {
    headline: string;
    slices: ApprovalRateSlices;
  };
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
