"use client";

import { useMemo } from "react";
import { ApprovalRateRing } from "@/components/skycot/approvals/ApprovalRateRing";
import { ConfidenceDonut } from "@/components/skycot/approvals/ConfidenceDonut";
import { DollarAtRisk } from "@/components/skycot/approvals/DollarAtRisk";
import { KpiStrip, type KpiCell } from "@/components/skycot/approvals/KpiStrip";
import { MetricStatCard } from "@/components/skycot/approvals/MetricStatCard";
import { SeverityPill, type Severity } from "@/components/skycot/approvals/SeverityPill";
import { SLABar } from "@/components/skycot/approvals/SLABar";
import { sortBySeverityThenSla, type ApprovalItem } from "@/lib/sample-approvals";
import { cn } from "@/lib/utils";

interface ApprovalsOpsDashboardProps {
  items: ApprovalItem[];
}

type SeverityTier = {
  severity: Severity;
  label: string;
  deck: string;
};

type ApprovalWithActionId = ApprovalItem & { actionId?: string };

const SAMPLE_APPROVAL_ACTION_IDS: Record<string, string> = {
  ap_01H9Z7: "act_018f67d0-0000-7000-8000-000000000001_018",
  ap_01H9Z6: "act_018f67d0-0000-7000-8000-000000000002_001",
  ap_01H9Z5: "act_018f67d0-0000-7000-8000-000000000003_027",
  ap_01H9Z4: "act_018f67d0-0000-7000-8000-000000000001_023",
  ap_01H9Z3: "act_018f67d0-0000-7000-8000-000000000004_032",
  ap_01H9Z2: "act_018f67d0-0000-7000-8000-000000000005_013",
  ap_01H9Z1: "act_018f67d0-0000-7000-8000-000000000002_002",
  ap_01H9Z0: "act_01J0NORI_001",
};

type ApprovalRateSummary = {
  total: number;
  approved: number;
  refused: number;
  expired: number;
};

const TIERS: readonly SeverityTier[] = [
  { severity: "p0", label: "High", deck: "Over SLA or high-risk decisions" },
  { severity: "p1", label: "Medium", deck: "Elevated work still inside SLA" },
  { severity: "p2", label: "Low", deck: "Routine confirmations" },
];

function formatUsd(amount: number): string {
  if (amount >= 1_000_000) return `$${(amount / 1_000_000).toFixed(1)}M`;
  if (amount >= 10_000) return `$${Math.round(amount / 1_000)}k`;
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    maximumFractionDigits: 0,
  }).format(amount);
}

function formatDuration(ms: number): string {
  const minutes = Math.max(0, Math.round(ms / 60_000));
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  const rem = minutes % 60;
  return rem === 0 ? `${hours}h` : `${hours}h ${rem}m`;
}

function median(values: number[]): number {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  if (sorted.length % 2 === 1) return sorted[mid] ?? 0;
  return ((sorted[mid - 1] ?? 0) + (sorted[mid] ?? 0)) / 2;
}

function severityLabel(severity: Severity): string {
  if (severity === "p0") return "High";
  if (severity === "p1") return "Medium";
  return "Low";
}

function workerInitials(name: string): string {
  const letters = name
    .split(/\s+/)
    .filter(Boolean)
    .map((part) => part[0])
    .join("");
  return (letters || name.slice(0, 2)).slice(0, 2).toUpperCase();
}

function computeApprovalRate(items: ApprovalItem[]): ApprovalRateSummary | null {
  const decided = items.filter((item) => item.decision != null);
  if (decided.length === 0) return null;
  const approved = decided.filter((item) => item.decision === "approve").length;
  const refused = decided.filter((item) => item.decision === "refuse").length;
  return {
    total: decided.length,
    approved: approved / decided.length,
    refused: refused / decided.length,
    expired: 0,
  };
}

function formatPercent(value: number): string {
  return `${Math.round(value * 100)}%`;
}

function computeKpis(
  pending: ApprovalItem[],
  approvalRate: ApprovalRateSummary | null,
): KpiCell[] {
  const now = Date.now();
  const atRisk = pending.reduce((sum, item) => sum + (item.dollarAtRisk ?? 0), 0);
  const monetary = pending.filter((item) => (item.dollarAtRisk ?? 0) > 0).length;
  const ages = pending.map((item) => now - new Date(item.receivedAt).getTime());
  return [
    {
      label: "Pending",
      value: String(pending.length),
      sub: "active human decisions",
      tone: pending.some((item) => item.severity === "p0") ? "bad" : "neutral",
      testId: "pending",
    },
    {
      label: "$ at risk",
      value: atRisk > 0 ? formatUsd(atRisk) : "—",
      sub: monetary > 0 ? `across ${monetary} pending` : "no monetary exposure",
      tone: atRisk > 0 ? "warn" : "neutral",
      testId: "at-risk",
    },
    {
      label: "Median time",
      value: formatDuration(median(ages)),
      sub: "waiting for approval",
      testId: "median-time",
    },
    {
      label: "Approval rate",
      value: approvalRate ? formatPercent(approvalRate.approved) : "—",
      sub: approvalRate
        ? `${approvalRate.total} resolved ${approvalRate.total === 1 ? "approval" : "approvals"}`
        : "no resolved approvals",
      tone: approvalRate ? "good" : "neutral",
      testId: "approval-rate",
    },
    {
      label: "Active reviewers",
      value: "—",
      sub: "reviewer roster pending",
      testId: "active-reviewers",
    },
  ];
}

function averageConfidence(items: ApprovalItem[]): number {
  const values = items.flatMap((item) =>
    typeof item.confidence === "number" ? [item.confidence] : [],
  );
  if (values.length === 0) return 0;
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

export function ApprovalsOpsDashboard({ items }: ApprovalsOpsDashboardProps) {
  const pending = useMemo(
    () => sortBySeverityThenSla(items.filter((item) => item.decision == null)),
    [items],
  );
  const approvalRate = useMemo(() => computeApprovalRate(items), [items]);
  const kpis = useMemo(
    () => computeKpis(pending, approvalRate),
    [pending, approvalRate],
  );
  const totalAtRisk = pending.reduce((sum, item) => sum + (item.dollarAtRisk ?? 0), 0);
  const avgConfidence = averageConfidence(pending);
  const riskyCount = pending.filter((item) => (item.dollarAtRisk ?? 0) > 0).length;
  const atRiskSubtitle = `${riskyCount} pending approval${
    riskyCount === 1 ? "" : "s"
  } with monetary exposure in the current queue`;

  return (
    <div className="space-y-6" data-testid="approvals-ops-dashboard">
      <KpiStrip cells={kpis} />

      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_320px]">
        <div className="space-y-5">
          {TIERS.map((tier) => {
            const tierItems = pending.filter(
              (item) => (item.severity ?? "p2") === tier.severity,
            );
            return (
              <SeveritySection key={tier.severity} tier={tier} items={tierItems} />
            );
          })}
        </div>

        <aside className="grid gap-4 sm:grid-cols-2 xl:grid-cols-1">
          <MetricStatCard
            title="$ at risk"
            value={totalAtRisk > 0 ? formatUsd(totalAtRisk) : "—"}
            subtitle={atRiskSubtitle}
            testId="at-risk-donut"
            tone="risk"
          />
          <article className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] p-5">
            <div className="flex items-center justify-between gap-4">
              <div>
                <h2 className="text-sm font-semibold text-[var(--foreground)]">
                  Confidence
                </h2>
                <p className="mt-1 text-xs text-[var(--muted-foreground)]">
                  Mean worker confidence across pending rows
                </p>
              </div>
              <ConfidenceDonut value={avgConfidence || 0} size={58} animate />
            </div>
          </article>
          <article className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] p-5">
            <div className="flex items-center gap-4">
              {approvalRate ? (
                <ApprovalRateRing
                  approved={approvalRate.approved}
                  refused={approvalRate.refused}
                  expired={approvalRate.expired}
                  animate
                />
              ) : (
                <span
                  data-testid="approval-rate-empty"
                  className="grid size-16 place-items-center rounded-full bg-[var(--muted)] text-sm font-semibold text-[var(--muted-foreground)]"
                >
                  —
                </span>
              )}
              <div>
                <h2 className="text-sm font-semibold text-[var(--foreground)]">
                  Approval rate
                </h2>
                <p className="mt-1 text-xs text-[var(--muted-foreground)]">
                  {approvalRate
                    ? `${formatPercent(approvalRate.approved)} approved · ${formatPercent(approvalRate.refused)} refused · ${formatPercent(approvalRate.expired)} expired`
                    : "No resolved approvals yet"}
                </p>
              </div>
            </div>
          </article>
        </aside>
      </div>
    </div>
  );
}

function SeveritySection({
  tier,
  items,
}: {
  tier: SeverityTier;
  items: ApprovalItem[];
}) {
  return (
    <section
      aria-labelledby={`approval-tier-${tier.severity}`}
      className="space-y-2"
      data-testid={`severity-tier-${tier.severity}`}
    >
      <div className="flex flex-wrap items-center gap-2">
        <SeverityPill severity={tier.severity} />
        <h2
          id={`approval-tier-${tier.severity}`}
          className="text-xs font-bold uppercase tracking-[0.12em] text-[var(--foreground)]"
        >
          {tier.label}
        </h2>
        <span className="font-mono text-[11px] text-[var(--muted-foreground)]">
          {items.length} {items.length === 1 ? "approval" : "approvals"}
        </span>
        <span className="text-xs text-[var(--muted-foreground)]">{tier.deck}</span>
      </div>
      <div className="overflow-hidden rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)]">
        {items.length === 0 ? (
          <div className="px-4 py-5 text-sm text-[var(--muted-foreground)]">
            No {tier.label.toLowerCase()} severity approvals waiting.
          </div>
        ) : (
          <ul className="divide-y divide-[var(--border)]">
            {items.map((item) => (
              <li key={item.id}>
                <ApprovalRow item={item} />
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}

function ApprovalRow({ item }: { item: ApprovalItem }) {
  const severity = item.severity ?? "p2";
  const actionId =
    (item as ApprovalWithActionId).actionId ?? SAMPLE_APPROVAL_ACTION_IDS[item.id] ?? item.id;
  const actionHref = `/audit/action/${encodeURIComponent(actionId)}`;
  return (
    <button
      type="button"
      data-testid={`recent-approval-${item.id}`}
      data-action-id={actionId}
      aria-label={`${item.worker.name} ${item.verb}`}
      onClick={() => {
        window.location.assign(actionHref);
      }}
      className={cn(
        "grid w-full cursor-pointer gap-3 px-4 py-4 text-left transition-colors",
        "hover:bg-[var(--muted)]/40 focus-visible:outline-none focus-visible:ring-2",
        "focus-visible:ring-[var(--ring)] focus-visible:ring-offset-2",
        "focus-visible:ring-offset-[var(--card)]",
        "lg:grid-cols-[minmax(0,1fr)_7.5rem_5rem_6.5rem]",
      )}
    >
      <div className="flex min-w-0 gap-3">
        <span
          aria-hidden
          className={cn(
            "grid size-9 shrink-0 place-items-center rounded-[var(--radius-md)]",
            "bg-[linear-gradient(135deg,var(--primary),var(--accent))]",
            "text-[11px] font-bold tracking-[0.06em] text-white",
          )}
        >
          {workerInitials(item.worker.name)}
        </span>
        <span className="min-w-0">
          <span className="block truncate text-sm font-semibold text-[var(--foreground)]">
            {item.verb}
          </span>
          <span className="mt-1 line-clamp-2 block text-xs text-[var(--muted-foreground)]">
            {item.worker.name} · {item.detail}
          </span>
          <span className="mt-2 inline-flex items-center gap-2 text-[11px] text-[var(--muted-foreground)]">
            <SeverityPill severity={severity} />
            {severityLabel(severity)}
          </span>
        </span>
      </div>
      <span className="self-center">
        {item.slaExpiresAt ? <SLABar expiresAt={item.slaExpiresAt} /> : null}
      </span>
      <span className="self-center lg:text-right">
        <DollarAtRisk amount={item.dollarAtRisk} currency={item.currency} />
      </span>
      <span className="self-center">
        {typeof item.confidence === "number" ? (
          <ConfidenceDonut value={item.confidence} size={28} />
        ) : (
          <span className="text-xs text-[var(--muted-foreground)]">No confidence</span>
        )}
      </span>
    </button>
  );
}
