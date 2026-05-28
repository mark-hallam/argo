"use client";

import { KpiStrip, type KpiCell } from "@/components/skycot/approvals/KpiStrip";
import type { ApprovalItem } from "@/lib/sample-approvals";

/**
 * Canonical approvals KPI strip — single source of truth for both
 * `/` (the root inbox) and `/approvals` (the ops-dashboard landing).
 *
 * Locked 5-cell label set per `project_approvals_page_design_locked`
 * + `fix-approvals-kpi-strip-label-set-converge-root-vs-approvals`:
 *
 *   1. PENDING            — count of pending approvals
 *   2. $ AT RISK          — sum of pending approval dollar values
 *   3. MEDIAN TIME        — median age of pending approvals (proxy
 *                            for time-to-decision; ApprovalItem does
 *                            not carry `decidedAt`)
 *   4. APPROVAL RATE      — % approved over resolved rows
 *   5. ACTIVE REVIEWERS   — distinct count of human reviewers
 *                            assigned to a pending row (from
 *                            `approver.name` on each pending item);
 *                            renders `—` + "no reviewers assigned"
 *                            when no pending row carries an approver
 */

export type ApprovalsKpis = {
  pendingCount: number;
  dollarsAtRisk: number;
  monetaryPendingCount: number;
  medianTimeMinutes: number | null;
  /** True when any pending row is over SLA or carries P0 severity. */
  hasUrgentPending: boolean;
  approvalRate: ApprovalRateSummary | null;
  /** Distinct count of human reviewers assigned to a pending row. */
  distinctApproverCount: number;
};

export type ApprovalRateSummary = {
  total: number;
  approvedFraction: number;
  refusedFraction: number;
  expiredFraction: number;
};

function formatUsd(amount: number): string {
  if (amount >= 1_000_000) return `$${(amount / 1_000_000).toFixed(1)}M`;
  if (amount >= 10_000) return `$${Math.round(amount / 1_000)}k`;
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    maximumFractionDigits: 0,
  }).format(amount);
}

function formatDuration(minutes: number): string {
  if (minutes < 60) return `${Math.max(0, Math.round(minutes))}m`;
  const hours = Math.floor(minutes / 60);
  const rem = Math.round(minutes % 60);
  return rem === 0 ? `${hours}h` : `${hours}h ${rem}m`;
}

function formatPercent(value: number): string {
  return `${Math.round(value * 100)}%`;
}

function median(values: number[]): number {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  if (sorted.length % 2 === 1) return sorted[mid] ?? 0;
  return ((sorted[mid - 1] ?? 0) + (sorted[mid] ?? 0)) / 2;
}

function parseTimestamp(iso?: string | null): number | null {
  if (!iso) return null;
  const t = new Date(iso).getTime();
  return Number.isFinite(t) ? t : null;
}

export function computeApprovalsKpis(
  items: ApprovalItem[],
  now: number = Date.now(),
): ApprovalsKpis {
  const pendingItems = items.filter((i) => i.decision == null);
  const decidedItems = items.filter((i) => i.decision != null);
  const approved = decidedItems.filter((i) => i.decision === "approve").length;
  const refused = decidedItems.filter((i) => i.decision === "refuse").length;
  const dollarsAtRisk = pendingItems.reduce(
    (sum, i) => sum + (i.dollarAtRisk ?? 0),
    0,
  );
  const monetaryPendingCount = pendingItems.filter(
    (i) => (i.dollarAtRisk ?? 0) > 0,
  ).length;
  const pendingAges = pendingItems
    .map((i) => {
      const received = parseTimestamp(i.receivedAt);
      return received == null ? null : (now - received) / 60_000;
    })
    .filter((v): v is number => v != null && v >= 0);
  const medianTimeMinutes =
    pendingAges.length > 0 ? median(pendingAges) : null;
  const hasUrgentPending = pendingItems.some((i) => {
    if (i.severity === "p0") return true;
    const expires = parseTimestamp(i.slaExpiresAt);
    return expires != null && expires <= now;
  });
  const approvalRate: ApprovalRateSummary | null =
    decidedItems.length === 0
      ? null
      : {
          total: decidedItems.length,
          approvedFraction: approved / decidedItems.length,
          refusedFraction: refused / decidedItems.length,
          expiredFraction: 0,
        };
  const distinctApproverCount = new Set(
    pendingItems
      .map((i) => i.approver?.name?.trim())
      .filter((name): name is string => Boolean(name)),
  ).size;
  return {
    pendingCount: pendingItems.length,
    dollarsAtRisk,
    monetaryPendingCount,
    medianTimeMinutes,
    hasUrgentPending,
    approvalRate,
    distinctApproverCount,
  };
}

export function buildApprovalsKpiCells(kpis: ApprovalsKpis): KpiCell[] {
  return [
    {
      label: "Pending",
      value: String(kpis.pendingCount),
      sub: "active human decisions",
      tone: kpis.hasUrgentPending ? "bad" : "neutral",
      testId: "pending",
    },
    {
      label: "$ at risk",
      value: kpis.dollarsAtRisk > 0 ? formatUsd(kpis.dollarsAtRisk) : "—",
      sub:
        kpis.monetaryPendingCount > 0
          ? `across ${kpis.monetaryPendingCount} pending`
          : "no monetary exposure",
      tone: kpis.dollarsAtRisk > 0 ? "warn" : "neutral",
      testId: "at-risk",
    },
    {
      label: "Median time",
      value:
        kpis.medianTimeMinutes == null
          ? "—"
          : formatDuration(kpis.medianTimeMinutes),
      sub:
        kpis.medianTimeMinutes == null
          ? "no pending approvals"
          : "waiting for approval",
      testId: "median-time",
    },
    {
      label: "Approval rate",
      value: kpis.approvalRate
        ? formatPercent(kpis.approvalRate.approvedFraction)
        : "—",
      sub: kpis.approvalRate
        ? `${kpis.approvalRate.total} resolved ${kpis.approvalRate.total === 1 ? "approval" : "approvals"}`
        : "no resolved approvals",
      tone: kpis.approvalRate ? "good" : "neutral",
      testId: "approval-rate",
    },
    {
      label: "Active reviewers",
      value:
        kpis.distinctApproverCount > 0
          ? String(kpis.distinctApproverCount)
          : "—",
      sub:
        kpis.distinctApproverCount > 0
          ? "reviewing now"
          : "no reviewers assigned",
      testId: "active-reviewers",
    },
  ];
}

export function ApprovalsKpiStrip({ kpis }: { kpis: ApprovalsKpis }) {
  return <KpiStrip cells={buildApprovalsKpiCells(kpis)} />;
}
