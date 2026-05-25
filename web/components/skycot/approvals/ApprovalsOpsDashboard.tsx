"use client";

import { useMemo } from "react";
import { ApprovalRateRing } from "@/components/skycot/approvals/ApprovalRateRing";
import { ConfidenceDonut } from "@/components/skycot/approvals/ConfidenceDonut";
import { DollarAtRisk } from "@/components/skycot/approvals/DollarAtRisk";
import { KpiStrip, type KpiCell } from "@/components/skycot/approvals/KpiStrip";
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

function computeKpis(items: ApprovalItem[], pending: ApprovalItem[]): KpiCell[] {
  const now = Date.now();
  const atRisk = pending.reduce((sum, item) => sum + (item.dollarAtRisk ?? 0), 0);
  const monetary = pending.filter((item) => (item.dollarAtRisk ?? 0) > 0).length;
  const ages = pending.map((item) => now - new Date(item.receivedAt).getTime());
  const decided = items.filter((item) => item.decision != null);
  const approved = decided.filter((item) => item.decision === "approve").length;
  const approvalRate = decided.length > 0 ? approved / decided.length : 0.78;
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
      value: `${Math.round(approvalRate * 100)}%`,
      sub: decided.length > 0 ? "from resolved rows" : "7-day baseline",
      tone: "good",
      testId: "approval-rate",
    },
    {
      label: "Active reviewers",
      value: pending.length > 0 ? "1" : "0",
      sub: "operator queue",
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
  const kpis = useMemo(() => computeKpis(items, pending), [items, pending]);
  const totalAtRisk = pending.reduce((sum, item) => sum + (item.dollarAtRisk ?? 0), 0);
  const avgConfidence = averageConfidence(pending);
  const riskyCount = pending.filter((item) => (item.dollarAtRisk ?? 0) > 0).length;

  return (
    <div className="space-y-6" data-testid="approvals-ops-dashboard">
      <KpiStrip
        cells={kpis}
        rate={{
          headline: "78%",
          slices: { approved: 0.78, refused: 0.14, expired: 0.08 },
          animate: true,
        }}
      />

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
          <MetricDonutCard
            title="$ at risk"
            value={totalAtRisk > 0 ? formatUsd(totalAtRisk) : "—"}
            subtitle={`${riskyCount} pending with monetary exposure`}
            testId="at-risk-donut"
            tone="risk"
            percent={totalAtRisk > 0 ? 0.82 : 0}
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
              <ApprovalRateRing approved={0.78} refused={0.14} expired={0.08} animate />
              <div>
                <h2 className="text-sm font-semibold text-[var(--foreground)]">
                  7-day approval rate
                </h2>
                <p className="mt-1 text-xs text-[var(--muted-foreground)]">
                  78% approved · 14% refused · 8% expired
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
  return (
    <button
      type="button"
      data-testid={`recent-approval-${item.id}`}
      aria-label={`${item.worker.name} ${item.verb}`}
      className={cn(
        "grid w-full gap-3 px-4 py-4 text-left transition-colors",
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

function MetricDonutCard({
  title,
  value,
  subtitle,
  testId,
  tone,
  percent,
}: {
  title: string;
  value: string;
  subtitle: string;
  testId: string;
  tone: "risk";
  percent: number;
}) {
  const color = tone === "risk" ? "var(--warning)" : "var(--primary)";
  const pct = Math.max(0, Math.min(1, percent));
  return (
    <article
      className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] p-5"
      data-testid={testId}
    >
      <div className="flex items-center gap-4">
        <span
          aria-hidden
          className="grid size-16 shrink-0 place-items-center rounded-full"
          style={{
            background: `conic-gradient(${color} 0 ${pct * 100}%, var(--muted) ${pct * 100}% 100%)`,
          }}
        >
          <span className="size-11 rounded-full bg-[var(--card)]" />
        </span>
        <div>
          <h2 className="text-sm font-semibold text-[var(--foreground)]">{title}</h2>
          <div className="mt-1 font-mono text-2xl font-semibold tabular-nums text-[var(--foreground)]">
            {value}
          </div>
          <p className="mt-1 text-xs text-[var(--muted-foreground)]">{subtitle}</p>
        </div>
      </div>
    </article>
  );
}
