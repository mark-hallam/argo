"use client";

import { useCallback, useMemo, type KeyboardEvent } from "react";
import { ApprovalRateRing } from "@/components/skycot/approvals/ApprovalRateRing";
import { ApprovalRowActions } from "@/components/skycot/approvals/ApprovalRowActions";
import { ApprovalsKpiStrip } from "@/components/skycot/approvals/ApprovalsKpiStrip";
import { ChannelChip } from "@/components/skycot/approvals/ChannelChip";
import { ConfidenceDonut } from "@/components/skycot/approvals/ConfidenceDonut";
import { DollarAtRisk } from "@/components/skycot/approvals/DollarAtRisk";
import { MetricStatCard } from "@/components/skycot/approvals/MetricStatCard";
import { SeverityPill, type Severity } from "@/components/skycot/approvals/SeverityPill";
import { SLABar } from "@/components/skycot/approvals/SLABar";
import {
  useSubmitDecision,
  useUndoDecision,
} from "@/lib/hooks/use-approvals";
import { useApprovalsKpis } from "@/lib/hooks/use-approvals-kpis";
import {
  sortBySeverityThenSla,
  type ApprovalItem,
  type ChannelKind,
} from "@/lib/sample-approvals";
import { cn } from "@/lib/utils";

interface ApprovalsOpsDashboardProps {
  items: ApprovalItem[];
}

type SeverityTier = {
  severity: Severity;
  label: string;
  deck: string;
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

function averageConfidence(items: ApprovalItem[]): number {
  const values = items.flatMap((item) =>
    typeof item.confidence === "number" ? [item.confidence] : [],
  );
  if (values.length === 0) return 0;
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

const CHANNEL_SUMMARY_LABEL: Record<ChannelKind, string> = {
  slack: "Slack",
  discord: "Discord",
  email: "Email",
  imessage: "iMessage",
  sms: "SMS",
  push: "Push",
  web: "Web",
  voice: "Voice",
  whatsapp: "WhatsApp",
  x: "X",
  linkedin: "LinkedIn",
};

function channelCounts(items: ApprovalItem[]): Array<[ChannelKind, number]> {
  const counts = new Map<ChannelKind, number>();
  for (const item of items) {
    counts.set(item.channel, (counts.get(item.channel) ?? 0) + 1);
  }
  return [...counts.entries()].sort((a, b) => b[1] - a[1]);
}

export function ApprovalsOpsDashboard({ items }: ApprovalsOpsDashboardProps) {
  const pending = useMemo(
    () => sortBySeverityThenSla(items.filter((item) => item.decision == null)),
    [items],
  );
  const approvalRate = useMemo(() => computeApprovalRate(items), [items]);
  const kpis = useApprovalsKpis(items);
  const totalAtRisk = pending.reduce((sum, item) => sum + (item.dollarAtRisk ?? 0), 0);
  const avgConfidence = averageConfidence(pending);
  const riskyCount = pending.filter((item) => (item.dollarAtRisk ?? 0) > 0).length;
  const pendingByChannel = useMemo(() => channelCounts(pending), [pending]);
  const atRiskSubtitle = `${riskyCount} pending approval${
    riskyCount === 1 ? "" : "s"
  } with monetary exposure in the current queue`;

  const submit = useSubmitDecision();
  const undo = useUndoDecision();

  // Brief: "remove the row from the pending list with a 5s Undo toast".
  // The optimistic update in `useSubmitDecision` already flips
  // `decision`, which removes the row from the `pending` derived list
  // above — no extra local state needed. The sonner toast wires the
  // Undo back-channel.
  //
  // NO-THEATER (day-1 unblock FIX 2): the toast now fires from
  // `onSuccess` with the gateway's actual `execution` outcome instead
  // of fire-and-forget at click time. Approve + "dispatched" → the
  // action really executed; approve + "failed" → the decision was
  // RECORDED but the dispatch failed, said destructively; `execution`
  // absent (older gateway / refuse / dev fallback) → the previous
  // generic toast. The Undo affordance stays on every variant.
  const handleDecide = useCallback(
    (id: string, decision: "approve" | "refuse") => {
      submit.mutate(
        { approvalId: id, decision },
        {
          onSuccess: (result) => {
            void import("sonner").then((m) => {
              const undoAction = {
                label: "Undo",
                onClick: () => undo.mutate(id),
              };
              if (decision === "approve" && result.execution === "dispatched") {
                m.toast.success("Approved — action executed", {
                  description: "See the audit trail.",
                  duration: 5000,
                  action: undoAction,
                });
                return;
              }
              if (decision === "approve" && result.execution === "failed") {
                m.toast.error("Approved, but execution failed", {
                  description: "The decision was recorded — check the audit trail.",
                  duration: 8000,
                  action: undoAction,
                });
                return;
              }
              m.toast.success(decision === "approve" ? "Approved" : "Refused", {
                duration: 5000,
                action: undoAction,
              });
            });
          },
        },
      );
    },
    [submit, undo],
  );

  const handleApprove = useCallback(
    (id: string) => handleDecide(id, "approve"),
    [handleDecide],
  );
  const handleRefuse = useCallback(
    (id: string) => handleDecide(id, "refuse"),
    [handleDecide],
  );

  return (
    <div className="space-y-6" data-testid="approvals-ops-dashboard">
      <ApprovalsKpiStrip kpis={kpis} />

      {pendingByChannel.length > 0 ? (
        <div
          data-testid="pending-by-channel"
          className="flex flex-wrap items-center gap-x-2 gap-y-1 text-[11.5px] text-[var(--muted-foreground)]"
        >
          <span className="font-semibold uppercase tracking-[0.1em] text-[10.5px]">
            Pending across channels
          </span>
          {pendingByChannel.map(([channel, count], idx) => (
            <span
              key={channel}
              data-testid={`pending-channel-${channel}`}
              className="inline-flex items-center gap-1 tabular-nums"
            >
              {idx > 0 ? <span aria-hidden>·</span> : null}
              <span>{CHANNEL_SUMMARY_LABEL[channel]}</span>
              <span className="font-semibold text-[var(--foreground)]">{count}</span>
            </span>
          ))}
        </div>
      ) : null}

      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_320px]">
        <div className="space-y-5">
          {TIERS.map((tier) => {
            const tierItems = pending.filter(
              (item) => (item.severity ?? "p2") === tier.severity,
            );
            return (
              <SeveritySection
                key={tier.severity}
                tier={tier}
                items={tierItems}
                onApprove={handleApprove}
                onRefuse={handleRefuse}
                disabled={submit.isPending}
              />
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
  onApprove,
  onRefuse,
  disabled,
}: {
  tier: SeverityTier;
  items: ApprovalItem[];
  onApprove: (id: string) => void;
  onRefuse: (id: string) => void;
  disabled?: boolean;
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
                <ApprovalRow
                  item={item}
                  onApprove={onApprove}
                  onRefuse={onRefuse}
                  disabled={disabled}
                />
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}

function ApprovalRow({
  item,
  onApprove,
  onRefuse,
  disabled,
}: {
  item: ApprovalItem;
  onApprove: (id: string) => void;
  onRefuse: (id: string) => void;
  disabled?: boolean;
}) {
  const severity = item.severity ?? "p2";
  const actionId = item.actionId ?? null;
  if (!actionId && process.env.NODE_ENV !== "production") {
    // Dev-only signal that the server (or test seed) omitted action_id.
    // Drill-in is disabled rather than landing on AuditActionNotFound.
    // eslint-disable-next-line no-console
    console.warn(
      `[approvals] item ${item.id} has no action_id — drill-in disabled`,
    );
  }
  const drillIn = actionId
    ? () => {
        window.location.assign(`/audit/action/${encodeURIComponent(actionId)}`);
      }
    : null;
  const onKeyDown = drillIn
    ? (e: KeyboardEvent<HTMLDivElement>) => {
        if (e.target !== e.currentTarget) return;
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          drillIn();
        }
      }
    : undefined;
  const clickable = drillIn != null;
  return (
    <div
      role={clickable ? "button" : undefined}
      tabIndex={clickable ? 0 : undefined}
      data-testid={`recent-approval-${item.id}`}
      data-action-id={actionId ?? undefined}
      aria-label={`${item.worker.name} ${item.verb}`}
      onClick={drillIn ?? undefined}
      onKeyDown={onKeyDown}
      className={cn(
        "flex w-full flex-wrap items-center gap-x-4 gap-y-3 px-4 py-4 text-left transition-colors",
        clickable
          ? cn(
              "cursor-pointer hover:bg-[var(--muted)]/40",
              "focus-visible:outline-none focus-visible:ring-2",
              "focus-visible:ring-[var(--ring)] focus-visible:ring-offset-2",
              "focus-visible:ring-offset-[var(--card)]",
            )
          : "cursor-default",
      )}
    >
      {/* Flex (not grid) row: the content block keeps `min-w-0 flex-1`
          so it fills the column WIDTH at any container size. The
          earlier viewport-keyed `xl:grid-cols-[1fr 7.5rem 5rem 6.5rem
          auto]` template crushed this block to ~85px on /approvals
          because the dashboard reserves a 320px aside — at a 1280px
          viewport the row only has ~640px, and four fixed metadata
          tracks (~480px) starved the `1fr` content track. Flex tracks
          the real container width, so the text fills and the metadata
          + actions cluster (below) sits inline when there's room and
          wraps as a unit when there isn't — matching the chronological
          TODAY card. `basis-[16rem]` lets content claim the row before
          the cluster wraps. */}
      <div
        className="flex min-w-0 flex-1 basis-[16rem] gap-3"
        data-testid={`approval-content-col-${item.id}`}
      >
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
        <span className="min-w-0 flex-1">
          <span className="block truncate text-sm font-semibold text-[var(--foreground)]">
            {item.verb}
          </span>
          <span className="mt-1 line-clamp-2 break-words text-xs text-[var(--muted-foreground)]">
            {item.worker.name} · {item.detail}
          </span>
          <span className="mt-2 inline-flex flex-wrap items-center gap-2 text-[11px] text-[var(--muted-foreground)]">
            <SeverityPill severity={severity} />
            {severityLabel(severity)}
            <ChannelChip channel={item.channel} />
          </span>
        </span>
      </div>
      <div
        className="ml-auto flex flex-wrap items-center justify-end gap-x-4 gap-y-2"
        data-testid={`approval-meta-col-${item.id}`}
      >
        {item.slaExpiresAt ? (
          <span className="shrink-0">
            <SLABar expiresAt={item.slaExpiresAt} />
          </span>
        ) : null}
        <span className="shrink-0 text-right">
          <DollarAtRisk amount={item.dollarAtRisk} currency={item.currency} />
        </span>
        <span className="shrink-0">
          {typeof item.confidence === "number" ? (
            <ConfidenceDonut value={item.confidence} size={28} />
          ) : (
            <span className="text-xs text-[var(--muted-foreground)]">No confidence</span>
          )}
        </span>
        <span className="shrink-0">
          <ApprovalRowActions
            approvalId={item.id}
            severity={severity}
            onApprove={onApprove}
            onRefuse={onRefuse}
            disabled={disabled}
          />
        </span>
      </div>
    </div>
  );
}
